//! Kintsugi Control Room — Tauri shell.
//!
//! The app is a **dashboard, not a gate** (`kintsugi-interaction-design.md`): it
//! reads what the daemon and the append-only event log already decided and shows
//! it. Every command here is a thin wrapper over [`kintsugi_app`], the tested
//! data-binding engine — no decision logic lives in the webview process. The
//! frontend (`../ui`) binds these over `window.__TAURI__.core.invoke`.

// Hide the console window on Windows release builds.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use kintsugi_app::{EngineStatus, ProvenanceView, QueueRow, TimelineRow};

/// The audit timeline (read-only event log), newest `limit` rows.
#[tauri::command]
fn timeline(limit: usize) -> Result<Vec<TimelineRow>, String> {
    kintsugi_app::timeline(&kintsugi_daemon::default_db_path(), limit).map_err(|e| e.to_string())
}

/// The live approval queue (held commands), over IPC.
#[tauri::command]
fn queue() -> Result<Vec<QueueRow>, String> {
    kintsugi_app::queue().map_err(|e| e.to_string())
}

/// The provenance trail for a session (optionally evaluating a command's legs).
#[tauri::command]
fn provenance(session: String, command: Option<String>) -> Result<ProvenanceView, String> {
    kintsugi_app::provenance(&session, command.as_deref()).map_err(|e| e.to_string())
}

/// Resolve a held command from the dashboard (the rare in-app decision).
#[tauri::command]
fn resolve(id: String, allow: bool) -> Result<(), String> {
    kintsugi_app::resolve(&id, allow).map_err(|e| e.to_string())
}

/// Engine status for the window chrome.
#[tauri::command]
fn status() -> EngineStatus {
    kintsugi_app::status()
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            timeline, queue, provenance, resolve, status
        ])
        .run(tauri::generate_context!())
        .expect("error while running the Kintsugi Control Room");
}
