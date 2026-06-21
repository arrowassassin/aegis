//! Screens. Dashboard, Activity feed (search + filter + pagination) and
//! Settings (theme switch + searchable local-model downloader) are built in
//! full. The remaining surfaces reuse the same visual vocabulary via
//! `Placeholder` — fill them in following the `Feed` table pattern.

use dioxus::prelude::*;
use crate::state::{use_store, ModelStatus, Screen, FEED_PAGE_SIZE};
use crate::theme::{decision, Theme};
use crate::data;

const FADE: &str = "animation:kfade .3s ease;";

// ───────────────────────────── Dashboard ─────────────────────────────

#[component]
pub fn Dashboard() -> Element {
    let mut store = use_store();
    let held = store.held_resolved.read().is_none();

    let (icon_path, title, sub, bg, line, icon_bg, icon_color) = if held {
        ("M12 3l7 3v5c0 4-3 7-7 9-4-2-7-5-7-9V6z M12 8.5v4 M12 15.4v.5",
         "1 thing needs your review", "Everything else is guarded. This one is waiting on you.",
         "linear-gradient(100deg,rgba(255,93,93,.12),transparent)", "rgba(255,93,93,.4)",
         "rgba(255,93,93,.14)", "var(--red)")
    } else {
        ("M12 3l7 3v5c0 4-3 7-7 9-4-2-7-5-7-9V6z M9 12l2 2 4-4",
         "You're protected", "3 agents guarded · nothing needs you right now.",
         "linear-gradient(100deg,rgba(90,247,142,.08),transparent)", "rgba(90,247,142,.3)",
         "rgba(90,247,142,.13)", "var(--green)")
    };
    let summary = [("1,247", "allowed", "var(--green)"), ("31", "held", "var(--amber)"), ("6", "blocked", "var(--red)")];

    rsx! {
        div { style: "padding:30px 26px;max-width:720px;margin:0 auto;{FADE}",
            div { style: "border:1px solid {line};border-radius:16px;background:{bg};padding:22px 24px",
                div { style: "display:flex;align-items:center;gap:16px",
                    span { style: "display:inline-flex;align-items:center;justify-content:center;width:46px;height:46px;flex:none;border-radius:12px;background:{icon_bg}",
                        svg { view_box: "0 0 24 24", width: "24", height: "24", fill: "none", stroke: "{icon_color}", stroke_width: "1.7", stroke_linecap: "round", stroke_linejoin: "round",
                            path { d: "{icon_path}" }
                        }
                    }
                    div { style: "flex:1",
                        div { style: "font-size:19px;font-weight:700;letter-spacing:-.2px", "{title}" }
                        div { style: "font-size:13.5px;color:var(--dim);margin-top:3px", "{sub}" }
                    }
                    if held {
                        button { class: "kn-btn-gold",
                            style: "flex:none;font-family:inherit;font-size:13.5px;font-weight:600;color:#1a1206;background:var(--gold);border:none;border-radius:9px;padding:11px 18px;cursor:pointer",
                            onclick: move |_| store.screen.set(Screen::Held),
                            "Review"
                        }
                    }
                }
                div { style: "display:flex;gap:26px;margin-top:20px;padding-top:18px;border-top:1px solid var(--hair)",
                    for (val, lbl, color) in summary {
                        div {
                            span { style: "font-size:21px;font-weight:700;font-family:'IBM Plex Mono',monospace;color:{color}", "{val}" }
                            span { style: "font-size:13px;color:var(--dim);margin-left:7px", "{lbl}" }
                        }
                    }
                    span { style: "margin-left:auto;align-self:center;font-size:12px;color:var(--dim)", "today" }
                }
            }

            div { style: "margin-top:18px;border:1px solid var(--line);border-radius:14px;background:var(--panel);overflow:hidden",
                div { style: "display:flex;align-items:center;padding:15px 18px;border-bottom:1px solid var(--line)",
                    span { style: "font-size:14px;font-weight:700", "Recent activity" }
                    button { style: "margin-left:auto;font-family:inherit;font-size:12.5px;font-weight:600;color:var(--gold);background:none;border:none;cursor:pointer",
                        onclick: move |_| store.screen.set(Screen::Feed),
                        "See all →"
                    }
                }
                for a in data::activity() {
                    {
                        let (glyph, color) = decision(a.decision);
                        rsx! {
                            div { style: "display:flex;align-items:center;gap:14px;padding:14px 18px;border-bottom:1px solid var(--hair)",
                                span { style: "display:inline-flex;align-items:center;justify-content:center;width:24px;height:24px;border-radius:7px;flex:none;font-size:12px;font-weight:700;color:{color}", "{glyph}" }
                                div { style: "flex:1;min-width:0",
                                    div { style: "font-size:13.5px;color:var(--ink);line-height:1.4", "{a.summary}" }
                                    div { style: "font-size:11.5px;color:var(--dim);margin-top:1px", "{a.agent}" }
                                }
                                span { style: "font-family:'IBM Plex Mono',monospace;font-size:11.5px;color:var(--dim);flex:none", "{a.time}" }
                            }
                        }
                    }
                }
            }

            div { style: "margin-top:16px;display:flex;align-items:center;gap:10px;font-size:12.5px;color:var(--dim)",
                svg { view_box: "0 0 24 24", width: "15", height: "15", fill: "none", stroke: "var(--green)", stroke_width: "1.8", stroke_linecap: "round", stroke_linejoin: "round", style: "flex:none",
                    path { d: "M20 6L9 17l-5-5" }
                }
                span { "Everything is logged and reversible — nothing here is permanent." }
            }
        }
    }
}

// ───────────────────────────── Activity feed ─────────────────────────────

#[component]
pub fn Feed() -> Element {
    let mut store = use_store();
    let filter = *store.feed_filter.read();
    let search = store.feed_search.read().to_lowercase();
    let page = *store.feed_page.read();

    let rows: Vec<data::FeedRow> = data::feed()
        .into_iter()
        .filter(|r| match filter {
            "held" => r.decision == "held",
            "blocked" => r.decision == "blocked",
            "tainted" => r.taint.is_some(),
            _ => true,
        })
        .filter(|r| search.is_empty() || r.command.to_lowercase().contains(&search) || r.agent.to_lowercase().contains(&search))
        .collect();

    let total = rows.len();
    let pages = ((total + FEED_PAGE_SIZE - 1) / FEED_PAGE_SIZE).max(1);
    let page = page.min(pages).max(1);
    let start = (page - 1) * FEED_PAGE_SIZE;
    let end = (start + FEED_PAGE_SIZE).min(total);
    let slice: Vec<data::FeedRow> = rows[start..end].to_vec();
    let info = if total == 0 { "No matches".to_string() } else { format!("{}–{} of {}", start + 1, end, total) };

    let cols = "grid-template-columns:64px 1fr 130px 124px 150px 110px;gap:14px";
    let filters = [("all", "All"), ("held", "Held"), ("blocked", "Blocked"), ("tainted", "Tainted")];

    rsx! {
        div { style: "padding:26px;max-width:1180px;{FADE}",
            div { style: "display:flex;gap:8px;margin-bottom:16px;flex-wrap:wrap;align-items:center",
                div { style: "position:relative;flex:1;min-width:200px;max-width:300px",
                    svg { view_box: "0 0 24 24", width: "15", height: "15", fill: "none", stroke: "var(--dim)", stroke_width: "1.8", stroke_linecap: "round", stroke_linejoin: "round", style: "position:absolute;left:11px;top:50%;transform:translateY(-50%)",
                        circle { cx: "11", cy: "11", r: "7" }
                        path { d: "M21 21l-4-4" }
                    }
                    input { class: "kn-input", value: "{store.feed_search}", placeholder: "Search commands or agents…",
                        style: "width:100%;height:34px;border-radius:8px;border:1px solid var(--line);background:var(--panel);color:var(--ink);padding:0 12px 0 33px;font-family:inherit;font-size:12.5px",
                        oninput: move |e| { store.feed_search.set(e.value()); store.feed_page.set(1); },
                    }
                }
                for (id, label) in filters {
                    {
                        let active = filter == id;
                        let st = if active { "background:var(--gold);color:#1a1206;border-color:var(--gold)" } else { "background:var(--panel);color:var(--dim)" };
                        rsx! {
                            button { style: "font-family:inherit;font-size:12.5px;font-weight:600;border-radius:8px;padding:7px 14px;cursor:pointer;border:1px solid var(--line);{st}",
                                onclick: move |_| { store.feed_filter.set(id); store.feed_page.set(1); },
                                "{label}"
                            }
                        }
                    }
                }
                div { style: "margin-left:auto;display:flex;align-items:center;gap:8px;font-size:12px;color:var(--dim)",
                    span { style: "display:inline-flex;width:7px;height:7px;border-radius:50%;background:var(--green);animation:kpulse 1.6s infinite" }
                    "streaming live"
                }
            }

            div { style: "border:1px solid var(--line);border-radius:12px;background:var(--panel);overflow:hidden",
                div { style: "display:grid;{cols};padding:11px 18px;border-bottom:1px solid var(--line);font-size:10.5px;font-weight:600;letter-spacing:.6px;color:var(--dim);text-transform:uppercase",
                    span { "Time" } span { "Command" } span { "Agent" } span { "Risk" } span { "Taint" }
                    span { style: "text-align:right", "Decision" }
                }
                for r in slice {
                    {
                        let (glyph, color) = decision(r.decision);
                        let (risk_label, risk_st) = r.risk.map(data::risk_style).unwrap_or(("", ""));
                        rsx! {
                            div { style: "display:grid;{cols};padding:12px 18px;border-bottom:1px solid var(--hair);align-items:center",
                                span { style: "font-family:'IBM Plex Mono',monospace;font-size:11.5px;color:var(--dim)", "{r.time}" }
                                span { style: "font-family:'IBM Plex Mono',monospace;font-size:12.5px;color:var(--ink);min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap", "{r.command}" }
                                span { style: "font-size:12.5px;color:var(--dim)", "{r.agent}" }
                                span {
                                    if r.risk.is_some() {
                                        span { style: "font-size:11.5px;font-weight:600;border-radius:6px;padding:3px 9px;{risk_st}", "{risk_label}" }
                                    }
                                }
                                span {
                                    if let Some(t) = r.taint {
                                        span { style: "font-size:11.5px;font-weight:600;color:var(--amber);display:inline-flex;align-items:center;gap:5px", "⚠ tainted · {t}" }
                                    }
                                }
                                span { style: "display:inline-flex;align-items:center;gap:6px;justify-content:flex-end;font-size:12.5px;font-weight:600;color:{color}", "{glyph} {r.decision}" }
                            }
                        }
                    }
                }
                if total == 0 {
                    div { style: "padding:34px 18px;text-align:center;font-size:13px;color:var(--dim)", "No commands match your search." }
                }
                div { style: "display:flex;align-items:center;gap:12px;padding:12px 18px;border-top:1px solid var(--line)",
                    span { style: "font-size:12px;color:var(--dim)", "{info}" }
                    div { style: "margin-left:auto;display:flex;align-items:center;gap:10px",
                        span { style: "font-size:12px;color:var(--dim)", "Page {page} of {pages}" }
                        button { class: "kn-btn-ghost", style: "font-family:inherit;font-size:12px;font-weight:600;color:var(--ink);background:var(--panel2);border:1px solid var(--line);border-radius:7px;padding:6px 12px;cursor:pointer",
                            onclick: move |_| { let p = *store.feed_page.read(); if p > 1 { store.feed_page.set(p - 1); } },
                            "‹ Prev"
                        }
                        button { class: "kn-btn-ghost", style: "font-family:inherit;font-size:12px;font-weight:600;color:var(--ink);background:var(--panel2);border:1px solid var(--line);border-radius:7px;padding:6px 12px;cursor:pointer",
                            onclick: move |_| { store.feed_page.set(*store.feed_page.read() + 1); },
                            "Next ›"
                        }
                    }
                }
            }
        }
    }
}

// ───────────────────────────── Settings ─────────────────────────────

#[component]
fn Toggle(on: bool, on_click: EventHandler<()>) -> Element {
    let track = if on { "background:var(--gold)" } else { "background:var(--line)" };
    let knob = if on { "left:21px" } else { "left:3px" };
    rsx! {
        button { style: "flex:none;width:42px;height:24px;border-radius:13px;border:none;cursor:pointer;position:relative;transition:background .15s;{track}",
            onclick: move |_| on_click.call(()),
            span { style: "position:absolute;top:3px;width:18px;height:18px;border-radius:50%;background:#fff;transition:left .15s;{knob}" }
        }
    }
}

#[component]
pub fn Settings() -> Element {
    let mut store = use_store();
    let theme = *store.theme.read();
    let dark_st = if theme == Theme::Dark { "background:var(--gold);color:#1a1206;border-color:var(--gold)" } else { "background:var(--panel2);color:var(--dim)" };
    let light_st = if theme == Theme::Light { "background:var(--gold);color:#1a1206;border-color:var(--gold)" } else { "background:var(--panel2);color:var(--dim)" };

    let model_status = *store.model_status.read();
    let search = store.model_search.read().to_lowercase();
    let progress = *store.model_progress.read();
    let active_id = *store.model_id.read();

    let toggles = [
        ("Fail-closed", "If the daemon is unreachable, block rather than run unguarded.", store.fail_closed),
        ("Auto-restart watchdog", "Run under systemd / launchd with restart-always.", store.watchdog),
        ("Passive session recording", "Log every human command to the tamper-evident audit trail.", store.recording),
        ("Require password to stop", "Stopping or disabling Kintsugi needs the admin password.", store.require_pw),
        ("Start on login", "Bring the daemon up automatically when the machine boots.", store.autostart),
    ];

    rsx! {
        div { style: "padding:26px;max-width:880px;{FADE}",
            // admin lock
            div { style: "border:1px solid var(--gold-line);border-radius:12px;background:linear-gradient(100deg,rgba(212,175,55,.06),transparent);padding:18px 20px;margin-bottom:16px;display:flex;align-items:center;gap:15px",
                span { style: "display:inline-flex;align-items:center;justify-content:center;width:40px;height:40px;border-radius:10px;background:rgba(212,175,55,.13);flex:none",
                    svg { view_box: "0 0 24 24", width: "20", height: "20", fill: "none", stroke: "var(--gold)", stroke_width: "1.7", stroke_linecap: "round", stroke_linejoin: "round",
                        rect { x: "4", y: "11", width: "16", height: "9", rx: "2" }
                        path { d: "M8 11V8a4 4 0 0 1 8 0v3" }
                    }
                }
                div { style: "flex:1",
                    div { style: "font-size:14px;font-weight:700", "Settings sealed · argon2id" }
                    div { style: "font-size:12.5px;color:var(--dim);margin-top:2px", "Loosening Kintsugi requires the admin password — enforced daemon-side with brute-force lockout." }
                }
                span { style: "font-size:11.5px;font-weight:600;color:var(--gold);border:1px solid var(--gold-line);border-radius:7px;padding:6px 11px;white-space:nowrap", "Locked" }
            }

            // appearance + lock
            div { style: "border:1px solid var(--line);border-radius:12px;background:var(--panel);padding:16px 20px;margin-bottom:16px;display:flex;align-items:center;gap:18px;flex-wrap:wrap",
                div { style: "flex:1;min-width:180px",
                    div { style: "font-size:13.5px;font-weight:600", "Appearance" }
                    div { style: "font-size:12px;color:var(--dim);margin-top:2px", "Choose a light or dark interface." }
                }
                div { style: "display:flex;gap:6px;background:var(--panel2);border:1px solid var(--line);border-radius:10px;padding:4px",
                    button { style: "font-family:inherit;font-size:12.5px;font-weight:600;border:1px solid transparent;border-radius:7px;padding:7px 14px;cursor:pointer;{dark_st}",
                        onclick: move |_| store.theme.set(Theme::Dark), "Dark" }
                    button { style: "font-family:inherit;font-size:12.5px;font-weight:600;border:1px solid transparent;border-radius:7px;padding:7px 14px;cursor:pointer;{light_st}",
                        onclick: move |_| store.theme.set(Theme::Light), "Light" }
                }
                div { style: "width:1px;height:34px;background:var(--line)" }
                button { class: "kn-btn-ghost", style: "display:inline-flex;align-items:center;gap:8px;font-family:inherit;font-size:12.5px;font-weight:600;color:var(--ink);background:var(--panel2);border:1px solid var(--line);border-radius:9px;padding:9px 15px;cursor:pointer",
                    onclick: move |_| store.lock(),
                    "Lock now"
                }
            }

            // local model downloader
            div { style: "border:1px solid var(--line);border-radius:12px;background:var(--panel);padding:18px 20px;margin-bottom:16px",
                div { style: "display:flex;align-items:flex-start;gap:13px;margin-bottom:14px",
                    span { style: "display:inline-flex;align-items:center;justify-content:center;width:38px;height:38px;border-radius:10px;background:rgba(212,175,55,.13);flex:none",
                        svg { view_box: "0 0 24 24", width: "20", height: "20", fill: "none", stroke: "var(--gold)", stroke_width: "1.6", stroke_linecap: "round", stroke_linejoin: "round",
                            rect { x: "5", y: "5", width: "14", height: "14", rx: "2" }
                            path { d: "M9 9h6v6H9zM9 2v3M15 2v3M9 19v3M15 19v3M2 9h3M2 15h3M19 9h3M19 15h3" }
                        }
                    }
                    div { style: "flex:1",
                        div { style: "font-size:14px;font-weight:700",
                            if model_status == ModelStatus::Installed { "Local model active" } else { "Heuristic scorer · offline" }
                        }
                        div { style: "font-size:12.5px;color:var(--dim);margin-top:2px;line-height:1.5",
                            "Kintsugi scores commands offline by default. Add an optional local model to sharpen the ambiguous band — it runs fully on-device and never phones home."
                        }
                    }
                }

                if model_status == ModelStatus::Installed {
                    if let Some(id) = active_id {
                        div { style: "border:1px solid var(--gold-line);border-radius:10px;background:rgba(212,175,55,.05);padding:13px 15px;margin-bottom:16px",
                            div { style: "font-size:11px;color:var(--dim);text-transform:uppercase;letter-spacing:.5px;margin-bottom:6px", "Pointing Kintsugi at it" }
                            div { style: "font-family:'IBM Plex Mono',monospace;font-size:12px;color:var(--ink);overflow-x:auto;white-space:nowrap",
                                "export KINTSUGI_MODEL_FILE=\"~/.local/share/kintsugi/models/{data::model_name(id)}-Q4_K_M.gguf\""
                            }
                            button { class: "kn-btn-ghost", style: "margin-top:10px;font-family:inherit;font-size:12px;font-weight:600;color:var(--dim);background:transparent;border:1px solid var(--line);border-radius:7px;padding:6px 12px;cursor:pointer",
                                onclick: move |_| { store.model_status.set(ModelStatus::None); store.model_id.set(None); store.model_progress.set(0.0); },
                                "Remove · back to heuristic"
                            }
                        }
                    }
                }

                div { style: "position:relative;margin-bottom:13px;width:260px;max-width:100%",
                    svg { view_box: "0 0 24 24", width: "14", height: "14", fill: "none", stroke: "var(--dim)", stroke_width: "1.8", stroke_linecap: "round", stroke_linejoin: "round", style: "position:absolute;left:10px;top:50%;transform:translateY(-50%)",
                        circle { cx: "11", cy: "11", r: "7" }
                        path { d: "M21 21l-4-4" }
                    }
                    input { class: "kn-input", value: "{store.model_search}", placeholder: "4B Instruct GGUF",
                        style: "width:100%;height:32px;border-radius:8px;border:1px solid var(--line);background:var(--panel2);color:var(--ink);padding:0 10px 0 31px;font-family:inherit;font-size:12px",
                        oninput: move |e| store.model_search.set(e.value()),
                    }
                }

                div { style: "display:flex;flex-direction:column;gap:9px",
                    for m in data::models().into_iter().filter(|m| search.is_empty() || m.id.to_lowercase().contains(&search)) {
                        {
                            let installed = model_status == ModelStatus::Installed && active_id == Some(m.id);
                            let downloading = model_status == ModelStatus::Downloading && active_id == Some(m.id);
                            let row_st = if installed { "border-color:var(--gold-line);background:rgba(212,175,55,.05)" }
                                else if downloading { "border-color:var(--gold-line)" } else { "" };
                            let name = data::model_name(m.id);
                            let publisher = data::model_publisher(m.id);
                            let pct = progress.round() as i32;
                            let fill = format!("width:{}%", if downloading { progress } else { 0.0 });
                            rsx! {
                                div { style: "display:flex;align-items:center;gap:13px;border:1px solid var(--line);border-radius:10px;background:var(--panel2);padding:12px 14px;{row_st}",
                                    div { style: "flex:1;min-width:0",
                                        div { style: "display:flex;align-items:center;gap:8px",
                                            if m.recommended { span { style: "font-size:12px;color:var(--gold)", "★" } }
                                            span { style: "font-family:'IBM Plex Mono',monospace;font-size:13px;font-weight:600;color:var(--ink);overflow:hidden;text-overflow:ellipsis;white-space:nowrap", "{name}" }
                                        }
                                        div { style: "font-size:11.5px;color:var(--dim);margin-top:3px", "{publisher} · {m.quant} · {m.size} · {m.downloads} downloads" }
                                    }
                                    if installed {
                                        span { style: "flex:none;font-size:12.5px;font-weight:600;color:var(--green);display:inline-flex;align-items:center;gap:6px",
                                            svg { view_box: "0 0 24 24", width: "14", height: "14", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linecap: "round", stroke_linejoin: "round", path { d: "M20 6L9 17l-5-5" } }
                                            "Installed"
                                        }
                                    } else if downloading {
                                        div { style: "flex:none;width:140px",
                                            div { style: "display:flex;justify-content:space-between;font-size:11px;color:var(--dim);margin-bottom:4px",
                                                span { "downloading" }
                                                span { style: "font-family:'IBM Plex Mono',monospace;color:var(--gold)", "{pct}%" }
                                            }
                                            div { style: "height:6px;border-radius:4px;background:var(--line);overflow:hidden",
                                                div { style: "height:100%;background:linear-gradient(90deg,var(--gold),var(--gold-bright));border-radius:4px;transition:width .15s;{fill}" }
                                            }
                                        }
                                    } else {
                                        button { class: "kn-btn-ghost", style: "flex:none;display:inline-flex;align-items:center;gap:7px;font-family:inherit;font-size:12.5px;font-weight:600;color:var(--ink);background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:8px 14px;cursor:pointer",
                                            onclick: move |_| start_download(store, m.id),
                                            svg { view_box: "0 0 24 24", width: "14", height: "14", fill: "none", stroke: "currentColor", stroke_width: "1.8", stroke_linecap: "round", stroke_linejoin: "round", path { d: "M12 3v12M7 11l5 5 5-5M5 21h14" } }
                                            "Download"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div { style: "font-size:11.5px;color:var(--dim);line-height:1.5;margin-top:13px;border-left:2px solid var(--gold);padding-left:12px",
                    "A model you pick is your choice — trusted because you selected it. The daemon never downloads on its own."
                }
            }

            // protection toggles
            div { style: "border:1px solid var(--line);border-radius:12px;background:var(--panel);overflow:hidden",
                for (label, desc, mut sig) in toggles {
                    div { style: "display:flex;align-items:center;gap:15px;padding:15px 20px;border-bottom:1px solid var(--hair)",
                        div { style: "flex:1",
                            div { style: "font-size:13.5px;font-weight:600", "{label}" }
                            div { style: "font-size:12px;color:var(--dim);margin-top:2px", "{desc}" }
                        }
                        Toggle { on: *sig.read(), on_click: move |_| { let v = *sig.read(); sig.set(!v); } }
                    }
                }
            }
        }
    }
}

/// Simulated streaming download — advances a progress signal, then installs.
fn start_download(mut store: crate::state::Store, id: &'static str) {
    store.model_status.set(ModelStatus::Downloading);
    store.model_id.set(Some(id));
    store.model_progress.set(0.0);
    spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(170)).await;
            let p = (*store.model_progress.read() + 9.0).min(100.0);
            store.model_progress.set(p);
            if p >= 100.0 {
                store.model_status.set(ModelStatus::Installed);
                break;
            }
        }
    });
}

// ───────────────────────────── Patterned screens ─────────────────────────────

/// The remaining surfaces (Held command, Provenance, Recorder, History,
/// Rules, Undo, and the three V2 plans) share the card / table / pill
/// vocabulary above. This renders a faithful shell for each so navigation and
/// look-and-feel are complete; port their bodies from the HTML using the
/// `Feed` table + card patterns.
#[component]
pub fn Placeholder() -> Element {
    let store = use_store();
    let (title, sub) = store.screen.read().meta();
    rsx! {
        div { style: "padding:26px;max-width:1000px;{FADE}",
            div { style: "border:1px solid var(--line);border-radius:14px;background:var(--panel);padding:30px 28px",
                div { style: "font-size:18px;font-weight:700;letter-spacing:-.2px", "{title}" }
                div { style: "font-size:13px;color:var(--dim);margin-top:6px;line-height:1.55;max-width:560px", "{sub}" }
                div { style: "margin-top:18px;display:inline-flex;align-items:center;gap:9px;font-size:12px;color:var(--gold);border:1px solid var(--gold-line);border-radius:8px;padding:8px 13px",
                    span { style: "width:7px;height:7px;border-radius:50%;background:var(--gold)" }
                    "Port this screen's body from the HTML using the Feed table + card patterns."
                }
            }
        }
    }
}
