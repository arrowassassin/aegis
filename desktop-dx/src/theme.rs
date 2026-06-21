//! Design tokens. Mirrors the original design's palette and the
//! `rootStyle` CSS-variable trick: one wrapper sets every `--token`, and all
//! components style themselves with `var(--token)`. Switching the theme is a
//! single string swap — no prop threading.

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

impl Theme {
    /// The CSS-variable block applied to the root element's `style`.
    pub fn root_vars(self) -> &'static str {
        match self {
            Theme::Dark => "\
--bg:#0b0e16;--bg2:#0e1320;--panel:#121a2b;--panel2:#0e1626;--term:#070a11;\
--line:#283557;--hair:rgba(40,53,87,.5);--ink:#e8ecf5;--dim:#8b95ad;\
--gold:#D4AF37;--gold-bright:#EBC65A;--gold-line:rgba(212,175,55,.35);\
--green:#5af78e;--amber:#ffd866;--red:#ff5d5d;--cyan:#6bd6ff;",
            Theme::Light => "\
--bg:#f3efe3;--bg2:#efe9da;--panel:#fbf8ef;--panel2:#f6f1e4;--term:#11151d;\
--line:#d8cfb6;--hair:rgba(201,192,166,.55);--ink:#1b2230;--dim:#6a7488;\
--gold:#9C6F1C;--gold-bright:#B8860B;--gold-line:rgba(156,111,28,.4);\
--green:#1f8a4c;--amber:#a9760a;--red:#c0392b;--cyan:#1f6f8b;",
        }
    }
}

/// Inline `style` for a decision pill / glyph. Returns (glyph, css-color).
pub fn decision(kind: &str) -> (&'static str, &'static str) {
    match kind {
        "allowed" => ("✓", "var(--green)"),
        "held" => ("❙", "var(--amber)"),
        "blocked" => ("✕", "var(--red)"),
        _ => ("◌", "var(--dim)"),
    }
}
