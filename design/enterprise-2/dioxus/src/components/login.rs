//! Master-password login gate. Mirrors the HTML unlock screen, gold seam and all.

use dioxus::prelude::*;
use crate::state::use_store;

#[component]
pub fn Login() -> Element {
    let mut store = use_store();
    let pw_error = *store.pw_error.read();
    let attempts = *store.attempts.read();
    let locked_out = attempts >= 5;
    let attempts_left = 5u32.saturating_sub(attempts);

    rsx! {
        div { style: "flex:1;display:flex;align-items:center;justify-content:center;position:relative;overflow:hidden",
            div { style: "position:absolute;inset:0;background:radial-gradient(680px 340px at 50% -4%,rgba(212,175,55,.07),transparent)" }
            div { style: "position:absolute;left:50%;top:0;bottom:0;width:1px;background:linear-gradient(var(--gold-bright),transparent 70%);opacity:.25" }

            div { style: "position:relative;width:380px;max-width:90%",
                div { style: "text-align:center",
                    img { src: crate::LOGO, width: "54", height: "54", alt: "Kintsugi",
                        style: "filter:drop-shadow(0 5px 18px rgba(212,175,55,.32))" }
                    div { style: "font-size:25px;font-weight:700;margin-top:15px;letter-spacing:-.3px", "Kintsugi" }
                    div { style: "font-size:13.5px;color:var(--dim);margin-top:5px", "Enter your master password to unlock" }
                }

                div { style: "margin-top:26px;position:relative",
                    input {
                        class: "kn-input",
                        r#type: "password",
                        value: "{store.pw}",
                        placeholder: "Master password",
                        autofocus: true,
                        style: "width:100%;height:50px;border-radius:12px;border:1px solid var(--line);background:var(--panel);color:var(--ink);padding:0 16px;font-family:inherit;font-size:14px",
                        oninput: move |e| { store.pw.set(e.value()); store.pw_error.set(false); },
                        onkeydown: move |e| { if e.key() == Key::Enter { store.try_unlock(); } },
                    }
                }

                if pw_error {
                    div { style: "display:flex;align-items:center;gap:8px;margin-top:11px;font-size:12.5px;color:var(--red)",
                        if locked_out { "Too many attempts — locked. Use your recovery key to reset." }
                        else { "That password is incorrect. Try again." }
                    }
                }
                if attempts > 0 && !locked_out {
                    div { style: "margin-top:8px;font-size:12px;color:var(--amber)", "{attempts_left} attempts left before lockout." }
                }

                button {
                    class: "kn-btn-gold",
                    style: "width:100%;margin-top:18px;height:48px;border:none;border-radius:12px;background:var(--gold);color:#1a1206;font-family:inherit;font-size:14.5px;font-weight:700;cursor:pointer",
                    onclick: move |_| store.try_unlock(),
                    "Unlock"
                }

                div { style: "margin-top:20px;text-align:center;font-size:11.5px;color:var(--dim);line-height:1.6",
                    "Sealed with argon2id · brute-force lockout after 5 tries"
                    br {}
                    span { style: "color:var(--gold)", "Demo password: " }
                    span { style: "font-family:'IBM Plex Mono',monospace;color:var(--ink)", "kintsugi" }
                }
            }
        }
    }
}
