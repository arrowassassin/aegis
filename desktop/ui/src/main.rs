//! Kintsugi Control Room — Dioxus (WASM) frontend.
//!
//! A dashboard, not a gate: it renders what the daemon and event log already
//! decided. Data arrives over Tauri `invoke`, deserialized into the shared
//! [`kintsugi_app_types`] view-models (the same types the native engine returns —
//! one compiler-checked contract, no npm, no hand-kept JSON shape).
//!
//! Design language carried from the codebase's TUI rules into the GUI: calm until
//! it must shout — one gold seam accent, the single danger accent reserved for a
//! trifecta block; every state pairs a glyph or word with color (never color
//! alone); mono for every command, path, and source id; a designed empty state.

use dioxus::prelude::*;
use kintsugi_app_types::{EngineStatus, ProvenanceView, TimelineRow};

mod invoke;
use invoke::invoke;

fn main() {
    dioxus::launch(App);
}

/// How often the dashboard re-polls the daemon for live updates (ms).
const POLL_MS: u32 = 1500;

#[component]
fn App() -> Element {
    // Selected timeline row (drives the detail/provenance pane).
    let mut selected = use_signal(|| None::<TimelineRow>);
    let filter = use_signal(String::new);

    // A tick that increments on a timer so the data resources re-fetch — live
    // updates without a restart, the non-blocking way (the await never freezes
    // the render loop).
    let mut tick = use_signal(|| 0u64);
    use_future(move || async move {
        loop {
            gloo_timers::future::TimeoutFuture::new(POLL_MS).await;
            tick += 1;
        }
    });

    let status = use_resource(move || async move {
        tick();
        invoke::<EngineStatus, _>("status", NoArgs {}).await.ok()
    });

    let rows = use_resource(move || async move {
        tick();
        invoke::<Vec<TimelineRow>, _>("timeline", TimelineArgs { limit: 200 })
            .await
            .unwrap_or_default()
    });

    let needle = filter().to_lowercase();
    let all = rows().unwrap_or_default();
    let visible: Vec<TimelineRow> = all
        .into_iter()
        .filter(|r| needle.is_empty() || r.command.to_lowercase().contains(&needle))
        .collect();

    rsx! {
        header { class: "topbar",
            div { class: "brand",
                span { class: "seam", aria_hidden: "true" }
                h1 { "Kintsugi" }
                span { class: "subtitle", "Control Room" }
            }
            StatusPills { status: status().flatten() }
        }
        main { class: "grid",
            Feed { rows: visible, selected, filter }
            Detail { selected, refresh: tick }
        }
    }
}

#[derive(serde::Serialize)]
struct TimelineArgs {
    limit: usize,
}

/// Serializes to `{}` for commands that take no arguments.
#[derive(serde::Serialize)]
struct NoArgs {}

#[component]
fn StatusPills(status: Option<EngineStatus>) -> Element {
    let (up, scorer) = match status {
        Some(s) => (s.running, s.scorer),
        None => (false, None),
    };
    rsx! {
        div { class: "pills",
            if up {
                span { class: "pill up", "● engine up" }
                if let Some(name) = scorer {
                    span { class: "pill", "scorer: {name}" }
                }
            } else {
                span { class: "pill down", "○ engine down" }
            }
        }
    }
}

#[component]
fn Feed(
    rows: Vec<TimelineRow>,
    selected: Signal<Option<TimelineRow>>,
    mut filter: Signal<String>,
) -> Element {
    let empty = rows.is_empty();
    rsx! {
        section { class: "panel feed", aria_label: "Command timeline",
            div { class: "panel-head",
                h2 { "Timeline" }
                input {
                    class: "filter",
                    r#type: "search",
                    placeholder: "filter commands…",
                    aria_label: "Filter the timeline",
                    value: "{filter}",
                    oninput: move |e| filter.set(e.value()),
                }
            }
            if empty {
                p { class: "empty",
                    "All quiet. Intercepted commands appear here as your agents work."
                }
            } else {
                ul { class: "rows", role: "list",
                    for row in rows {
                        Row { row, selected }
                    }
                }
            }
        }
    }
}

#[component]
fn Row(row: TimelineRow, selected: Signal<Option<TimelineRow>>) -> Element {
    let is_sel = selected().as_ref().map(|s| s.id == row.id).unwrap_or(false);
    let trifecta = row.provenance_block;
    let class = format!(
        "row{}{}",
        if is_sel { " selected" } else { "" },
        if trifecta { " trifecta" } else { "" }
    );
    let pick = row.clone();
    rsx! {
        li {
            class: "{class}",
            onclick: move |_| selected.set(Some(pick.clone())),
            span { class: "agent", "{row.agent}" }
            span { class: "cmd", "{row.command}" }
            span { class: "badge {row.outcome}", "{row.outcome}" }
        }
    }
}

#[component]
fn Detail(selected: Signal<Option<TimelineRow>>, refresh: Signal<u64>) -> Element {
    let Some(ev) = selected() else {
        return rsx! {
            section { class: "panel detail", aria_label: "Selected command",
                div { class: "empty", "Select a command to see why Kintsugi decided what it did." }
            }
        };
    };

    // Pull the provenance trail for this row's session (and command) live.
    let session = ev.session.clone();
    let command = ev.command.clone();
    let trail = use_resource(move || {
        let session = session.clone();
        let command = command.clone();
        async move {
            refresh();
            let Some(session) = session else {
                return None;
            };
            invoke::<ProvenanceView, _>(
                "provenance",
                ProvenanceArgs { session, command: Some(command) },
            )
            .await
            .ok()
        }
    });

    rsx! {
        section { class: "panel detail", aria_label: "Selected command",
            article {
                div { class: "decision",
                    span { class: "badge {ev.outcome}", "{ev.outcome}" }
                    span { class: "muted", "{ev.class}" }
                    if ev.provenance_block {
                        span { class: "badge trifecta", "⛔ lethal-trifecta" }
                    }
                }
                pre { class: "command", "{ev.command}" }
                dl { class: "meta",
                    dt { "agent" } dd { "{ev.agent}" }
                    dt { "session" } dd { "{ev.session.clone().unwrap_or_default()}" }
                    dt { "reason" } dd { "{ev.reason}" }
                }
                if let Some(Some(view)) = trail() {
                    if view.tainted && !view.trail.is_empty() {
                        Trail { view }
                    }
                }
            }
        }
    }
}

#[component]
fn Trail(view: ProvenanceView) -> Element {
    rsx! {
        section { class: "trail-wrap",
            h3 { "Provenance trail" }
            ol { class: "trail", aria_label: "How untrusted content reached this command",
                for step in view.trail {
                    {
                        let (glyph, label) = step.glyph_label();
                        let li_class = if step.is_rule() { "rule" } else { "" };
                        rsx! {
                            li { class: "{li_class}",
                                span { class: "dot", aria_hidden: "true" }
                                span { class: "glyph", "{glyph}" }
                                span { class: "step-label", "{label}  " }
                                span { class: "val", "{step.value()}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(serde::Serialize)]
struct ProvenanceArgs {
    session: String,
    command: Option<String>,
}
