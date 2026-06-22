# Kintsugi Control Room — Dioxus (Rust)

A faithful Rust/[Dioxus 0.6](https://dioxuslabs.com) port of the Kintsugi enterprise control-room design. The desktop target renders through a webview, so the original inline-style + CSS-variable design system carries over **1:1** — same palette, type, spacing, gold-seam motif, calm-by-default tone.

## Run

```bash
# one-time: install the Dioxus CLI (optional but recommended)
cargo install dioxus-cli

# from this `dioxus/` folder:
dx serve            # hot-reloading dev
# or
cargo run           # plain desktop window
```

Demo master password: **`kintsugi`**.

## How it maps to the original HTML design

| HTML design                         | Rust module            |
|-------------------------------------|------------------------|
| `rootStyle` CSS-variable palette    | `src/theme.rs`         |
| `this.state` (signals)              | `src/state.rs` — a `Store` of `Copy` `Signal`s shared via context |
| `renderVals()` data arrays          | `src/data.rs`          |
| Login gate                          | `src/components/login.rs` |
| Titlebar / collapsible sidebar / topbar | `src/components/shell.rs` |
| Screens                             | `src/components/screens.rs` |

The CSS-variable trick is the key to keeping the Rust small: one wrapper `<div>` sets every `--token`, and every component styles itself with `var(--token)`. Switching dark/light is a single string swap (`Theme::root_vars`), exactly like the original.

## What's fully built vs. patterned

**Built end-to-end** (logic + visuals):

- **Login** — master password, error + attempts-left states, 5-try lockout, gold seam.
- **Shell** — OS-neutral titlebar with hamburger, collapsible icon sidebar (248 ⇄ 64 px), topbar with lock + panic, panic banner.
- **Home** — calm status hero, plain-language activity, reassurance line.
- **Activity** — search + filter chips + pagination over the feed.
- **Settings** — dark/light theme switch, **searchable Hugging Face local-model downloader** (progress → installed → `KINTSUGI_MODEL_FILE`), protection toggles, admin lock, lock-now.

**Patterned** (`screens::Placeholder`): Held command, Provenance, Recorder, History, Rules, Undo, and the three V2 plans (Verified gate, Capability scopes, Team fleet). Each renders a correct shell so navigation and look-and-feel are complete — port their bodies from the HTML using the `Feed` table and Settings card patterns. They're deliberately left as a clear, repeatable template rather than 1,000 lines of generated tables.

## Notes for the porting dev

- **Pseudo-classes**: Dioxus inline styles can't carry `:hover`/`:focus`, so the few we rely on live as named classes in `assets/styles.css` (`.kn-btn-gold`, `.kn-input`, …). Add more there as needed.
- **Async**: the model download uses `spawn` + `tokio::time::sleep`. Swap for your runtime's timer if you retarget web/mobile.
- **Targets**: change the `dioxus` feature in `Cargo.toml` (`desktop` → `web` / `mobile`) to retarget; the UI code is unchanged.
- **Fonts**: IBM Plex Sans / Mono are pulled from Google Fonts in `styles.css`. For an offline desktop build, vendor the `.woff2` files and `@font-face` them instead.
