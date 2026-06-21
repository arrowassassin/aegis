//! Kintsugi Control Room — Dioxus desktop port of the HTML design.
//!
//! Run:  cargo run        (or: dx serve  with the dioxus CLI)
//!
//! Architecture mirrors the original Design Component:
//!   theme.rs   — palette as a CSS-variable block (the `rootStyle` trick)
//!   state.rs   — a `Store` of Copy signals shared via context (`this.state`)
//!   data.rs    — mock data (the arrays from `renderVals()`)
//!   components — login gate, shell (titlebar/sidebar/topbar), screens
//!
//! Because the desktop target renders through a webview, every inline style and
//! CSS variable from the HTML carries over unchanged — the look and feel is 1:1.

use dioxus::prelude::*;

mod theme;
mod state;
mod data;
mod components;

use state::{Store, Screen};
use components::{login::Login, shell::{TitleBar, Sidebar, TopBar}, screens};

pub const LOGO: Asset = asset!("/assets/logo-mark.svg");
pub const STYLES: Asset = asset!("/assets/styles.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    // Provide the shared store to the whole tree (context = `this`).
    use_context_provider(Store::new);
    let store = use_context::<Store>();

    let theme_vars = store.theme.read().root_vars();
    let unlocked = *store.unlocked.read();
    let panic = *store.panic.read();

    rsx! {
        document::Link { rel: "stylesheet", href: STYLES }

        div {
            style: "height:100vh;width:100%;display:flex;flex-direction:column;color:var(--ink);background:var(--bg);overflow:hidden;font-family:'IBM Plex Sans',ui-sans-serif,system-ui,sans-serif;{theme_vars}",

            if !unlocked {
                Login {}
            } else {
                TitleBar {}

                if panic { PanicBanner {} }

                div { style: "flex:1;display:flex;min-height:0",
                    Sidebar {}
                    main { style: "flex:1;min-width:0;display:flex;flex-direction:column;background:var(--bg)",
                        TopBar {}
                        div { style: "flex:1;overflow-y:auto;min-height:0",
                            ScreenRouter {}
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ScreenRouter() -> Element {
    let store = use_context::<Store>();
    match *store.screen.read() {
        Screen::Dashboard => rsx! { screens::Dashboard {} },
        Screen::Feed => rsx! { screens::Feed {} },
        Screen::Settings => rsx! { screens::Settings {} },
        // Held, Provenance, Recorder, Audit, Policy, Snapshots and the three
        // V2 plans share the card/table vocabulary — see screens::Placeholder.
        _ => rsx! { screens::Placeholder {} },
    }
}

#[component]
fn PanicBanner() -> Element {
    let mut store = use_context::<Store>();
    rsx! {
        div { style: "flex:none;display:flex;align-items:center;gap:14px;padding:11px 20px;background:linear-gradient(90deg,rgba(255,93,93,.18),rgba(255,93,93,.05));border-bottom:1px solid rgba(255,93,93,.4)",
            span { style: "display:inline-flex;width:9px;height:9px;border-radius:50%;background:var(--red);animation:kpulse 1.1s infinite" }
            span { style: "font-size:13.5px;font-weight:600;color:var(--red)", "Panic engaged — all agent actions halted and queued." }
            span { style: "font-size:13px;color:var(--dim)", "Nothing runs until you resume." }
            button { class: "kn-btn-ghost", style: "margin-left:auto;font-family:inherit;font-size:12.5px;font-weight:600;color:var(--ink);background:var(--panel);border:1px solid var(--line);border-radius:7px;padding:7px 14px;cursor:pointer",
                onclick: move |_| store.panic.set(false),
                "Resume guarding"
            }
        }
    }
}
