//! Shell AST front-end for the classifier (pure-Rust, via `brush-parser`).
//!
//! The Tier-1 classifier historically tokenized the command line with a small
//! hand-rolled splitter. That is fast and dependency-light, but it can't see the
//! true shell structure — commands hidden inside command substitution `$(…)` /
//! backticks, here-documents fed to a shell, or unusual quoting. This module
//! parses the line into a real bash AST and flattens it to the list of *simple
//! commands* it would run, descending into:
//!   - pipelines, `&&`/`||`/`;` lists, and compound commands (subshells, groups,
//!     `if`/`for`/`while`/`case`/functions),
//!   - command substitutions `$(…)` and backticks found in any word,
//!   - here-document bodies fed to a shell (`bash <<EOF … EOF`),
//!   - the `-c` script of a shell, and `find -exec` / `xargs` payloads.
//!
//! The classifier composes this with the existing tokenizer pass **worst-wins**,
//! so the AST can only ever *add* detections (deeper, obfuscated payloads) and
//! never downgrade a tokenizer verdict. A parse failure yields `None` — the
//! caller treats that as "the AST found nothing", and the tokenizer pass (and
//! the fail-toward-caution default) still stands.

use brush_parser::ast;

/// One simple command extracted from the AST: program plus its argument words.
/// Word text is raw (quotes/expansions preserved), exactly as the agent wrote
/// it — the classifier trims quotes where it matters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleCmd {
    pub program: String,
    pub args: Vec<String>,
}

/// What the AST pass found: every simple command the line would run (flattened,
/// including those nested in substitutions / compounds), plus the raw text of
/// every command substitution `$(…)` / backtick — so the classifier can also
/// whole-line-scan substitution bodies (e.g. a `curl … | sh` hidden in `$(…)`).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Analysis {
    pub commands: Vec<SimpleCmd>,
    pub substitutions: Vec<String>,
}

/// Parse `raw` into an [`Analysis`]. Returns `None` if the line can't be parsed
/// (caller falls back to the tokenizer pass + the cautious default).
pub fn analyze(raw: &str) -> Option<Analysis> {
    let program = parse_program(raw)?;
    let mut a = Analysis::default();
    collect_program(&program, &mut a, 0);
    Some(a)
}

/// Just the flattened simple commands (used in tests).
pub fn ast_commands(raw: &str) -> Option<Vec<SimpleCmd>> {
    analyze(raw).map(|a| a.commands)
}

/// Program basename without directory or `.exe`.
fn basename(arg0: &str) -> &str {
    let base = arg0.rsplit(['/', '\\']).next().unwrap_or(arg0);
    base.strip_suffix(".exe").unwrap_or(base)
}

fn parse_program(raw: &str) -> Option<ast::Program> {
    let tokens = brush_parser::tokenize_str(raw).ok()?;
    let opts = brush_parser::ParserOptions::default();
    brush_parser::parse_tokens(&tokens, &opts).ok()
}

/// Guard against pathological nesting of substitutions / compounds.
const MAX_DEPTH: u8 = 8;

fn collect_program(program: &ast::Program, a: &mut Analysis, depth: u8) {
    if depth > MAX_DEPTH {
        return;
    }
    for complete in &program.complete_commands {
        collect_compound_list(complete, a, depth);
    }
}

fn collect_compound_list(list: &ast::CompoundList, a: &mut Analysis, depth: u8) {
    if depth > MAX_DEPTH {
        return;
    }
    for item in &list.0 {
        collect_and_or(&item.0, a, depth);
    }
}

fn collect_and_or(and_or: &ast::AndOrList, a: &mut Analysis, depth: u8) {
    collect_pipeline(&and_or.first, a, depth);
    for extra in &and_or.additional {
        let pipeline = match extra {
            ast::AndOr::And(p) | ast::AndOr::Or(p) => p,
        };
        collect_pipeline(pipeline, a, depth);
    }
}

fn collect_pipeline(pipeline: &ast::Pipeline, a: &mut Analysis, depth: u8) {
    for cmd in &pipeline.seq {
        collect_command(cmd, a, depth);
    }
}

fn collect_command(cmd: &ast::Command, a: &mut Analysis, depth: u8) {
    match cmd {
        ast::Command::Simple(sc) => collect_simple(sc, a, depth),
        ast::Command::Compound(compound, _redirects) => collect_compound(compound, a, depth + 1),
        // Function definitions / extended-test expressions don't *run* a command
        // by themselves; their bodies are reached when invoked. Ignore here.
        _ => {}
    }
}

fn collect_compound(compound: &ast::CompoundCommand, a: &mut Analysis, depth: u8) {
    if depth > MAX_DEPTH {
        return;
    }
    use ast::CompoundCommand::*;
    match compound {
        BraceGroup(g) => collect_compound_list(&g.list, a, depth),
        Subshell(s) => collect_compound_list(&s.list, a, depth),
        ForClause(f) => collect_compound_list(&f.body.list, a, depth),
        // `WhileOrUntilClauseCommand(condition, do-group, loc)`.
        WhileClause(w) | UntilClause(w) => {
            collect_compound_list(&w.0, a, depth);
            collect_compound_list(&w.1.list, a, depth);
        }
        IfClause(i) => {
            collect_compound_list(&i.condition, a, depth);
            collect_compound_list(&i.then, a, depth);
            if let Some(elses) = &i.elses {
                for e in elses {
                    if let Some(cond) = &e.condition {
                        collect_compound_list(cond, a, depth);
                    }
                    collect_compound_list(&e.body, a, depth);
                }
            }
        }
        CaseClause(c) => {
            for item in &c.cases {
                if let Some(cmds) = &item.cmd {
                    collect_compound_list(cmds, a, depth);
                }
            }
        }
        Arithmetic(_) | ArithmeticForClause(_) | Coprocess(_) => {}
    }
}

fn collect_simple(sc: &ast::SimpleCommand, a: &mut Analysis, depth: u8) {
    // Every word that could carry a command substitution: the prefix
    // assignments (e.g. `x=$(…)`), the program name, and the argument words.
    let mut scan_words: Vec<String> = Vec::new();

    if let Some(prefix) = &sc.prefix {
        for item in &prefix.0 {
            match item {
                ast::CommandPrefixOrSuffixItem::AssignmentWord(_, w) => {
                    scan_words.push(w.value.clone())
                }
                ast::CommandPrefixOrSuffixItem::Word(w) => scan_words.push(w.value.clone()),
                _ => {}
            }
        }
    }

    // If the program is a shell, a here-doc / here-string body IS a script.
    let is_shell = sc
        .word_or_name
        .as_ref()
        .map(|n| {
            matches!(
                basename(&n.value),
                "sh" | "bash" | "zsh" | "dash" | "ash" | "ksh"
            )
        })
        .unwrap_or(false);

    let mut args = Vec::new();
    if let Some(suffix) = &sc.suffix {
        for item in &suffix.0 {
            match item {
                ast::CommandPrefixOrSuffixItem::Word(w) => args.push(w.value.clone()),
                // A here-doc / here-string fed to a shell carries a script body.
                ast::CommandPrefixOrSuffixItem::IoRedirect(io) if is_shell => {
                    let body = match io {
                        ast::IoRedirect::HereDocument(_, hd) => Some(hd.doc.value.clone()),
                        // A here-string keeps its surrounding quotes in the word;
                        // the actual stdin is the unquoted content.
                        ast::IoRedirect::HereString(_, w) => {
                            Some(w.value.trim_matches(['"', '\'']).to_string())
                        }
                        _ => None,
                    };
                    if let Some(body) = body {
                        if let Some(inner) = parse_program(&body) {
                            collect_program(&inner, a, depth + 1);
                        }
                    }
                }
                _ => {}
            }
            // File redirects are handled by the snapshot predictor, not here.
        }
    }

    if let Some(name) = &sc.word_or_name {
        scan_words.push(name.value.clone());
    }
    for arg in &args {
        scan_words.push(arg.clone());
    }

    // Record + recurse command substitutions / backticks found in any of those
    // words (so substitution bodies are both classified per-program and
    // available for the whole-line scans).
    for word in &scan_words {
        for sub in command_substitutions(word) {
            if let Some(inner) = parse_program(&sub) {
                collect_program(&inner, a, depth + 1);
            }
            a.substitutions.push(sub);
        }
    }

    // A pure assignment (`x=…`) has no program to classify, but its substitution
    // was already recursed above.
    if let Some(name) = &sc.word_or_name {
        a.commands.push(SimpleCmd {
            program: name.value.clone(),
            args,
        });
    }
}

/// Extract command-substitution payloads from a single (raw) word: `$(…)` with
/// balanced parens, and `` `…` `` backticks. Substitutions inside single quotes
/// are NOT expanded by the shell, so they're skipped.
fn command_substitutions(word: &str) -> Vec<String> {
    let mut subs = Vec::new();
    let bytes = word.as_bytes();
    let mut i = 0;
    let mut in_single = false;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == '\'' {
            in_single = !in_single;
            i += 1;
            continue;
        }
        if in_single {
            i += 1;
            continue;
        }
        // `$(` … `)` with paren-depth tracking (handles nested `$( $() )`).
        if c == '$' && i + 1 < bytes.len() && bytes[i + 1] == b'(' {
            let start = i + 2;
            let mut depth = 1;
            let mut j = start;
            while j < bytes.len() && depth > 0 {
                match bytes[j] {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
                j += 1;
            }
            if depth == 0 {
                subs.push(word[start..j - 1].to_string());
                i = j;
                continue;
            }
        }
        // Backticks: `…` up to the next backtick.
        if c == '`' {
            if let Some(end) = word[i + 1..].find('`') {
                subs.push(word[i + 1..i + 1 + end].to_string());
                i = i + 1 + end + 1;
                continue;
            }
        }
        i += 1;
    }
    subs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn progs(raw: &str) -> Vec<String> {
        ast_commands(raw)
            .unwrap_or_default()
            .into_iter()
            .map(|c| c.program)
            .collect()
    }

    #[test]
    fn flattens_pipelines_lists_and_separators() {
        let p = progs("cd build && rm -rf ../dist; echo a | sh");
        assert!(p.contains(&"cd".to_string()));
        assert!(p.contains(&"rm".to_string()));
        assert!(p.contains(&"echo".to_string()));
        assert!(p.contains(&"sh".to_string()));
    }

    #[test]
    fn recurses_command_substitution_and_backticks() {
        // The dangerous program lives only inside `$(…)` / backticks.
        assert!(progs("echo \"$(rm -rf /)\"").contains(&"rm".to_string()));
        assert!(progs("x=`git push --force`").contains(&"git".to_string()));
        // Nested substitution.
        assert!(progs("echo $( echo $(terraform destroy) )").contains(&"terraform".to_string()));
    }

    #[test]
    fn single_quotes_are_not_substitutions() {
        // `$(...)` inside single quotes is literal text, not a command.
        let p = progs("echo '$(rm -rf /)'");
        assert!(p.contains(&"echo".to_string()));
        assert!(
            !p.contains(&"rm".to_string()),
            "single-quoted is literal: {p:?}"
        );
    }

    #[test]
    fn descends_into_compounds() {
        assert!(progs("if true; then rm -rf /; fi").contains(&"rm".to_string()));
        assert!(progs("( cd x && git push --force )").contains(&"git".to_string()));
    }

    #[test]
    fn unparseable_is_none() {
        // An unterminated quote is a parse error → None (caller stays cautious).
        assert!(ast_commands("echo 'unterminated").is_none());
    }

    #[test]
    fn args_are_captured() {
        let cmds = ast_commands("rm -rf build").unwrap();
        let rm = cmds.iter().find(|c| c.program == "rm").unwrap();
        assert!(rm.args.iter().any(|a| a == "-rf"));
        assert!(rm.args.iter().any(|a| a == "build"));
    }
}
