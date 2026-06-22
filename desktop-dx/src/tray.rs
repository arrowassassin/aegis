//! System-tray status indicator: macOS menubar, Windows taskbar, Linux indicator.
//!
//! Shows the brand mark when Kintsugi is running, with a "Show Kintsugi" /
//! "Quit" menu and a left-click bringing the main window back to the front.
//! Optional — if the tray library fails to initialize (e.g. an embedded Linux
//! without AppIndicator), the app continues to work without a tray.

use std::sync::atomic::{AtomicBool, Ordering};

use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent};

/// Cross-process flag the Dioxus side polls each tick — true when the tray was
/// clicked (or "Show Kintsugi" was picked from the menu) and the main window
/// should be brought to the front. Reset to false after the UI handles it.
pub static SHOW_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Decode the embedded 32-px PNG into the RGBA buffer the tray library wants.
fn icon_image() -> Option<tray_icon::Icon> {
    let png = crate::ICON_PNG_32;
    let img = image::load_from_memory(png).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    tray_icon::Icon::from_rgba(img.into_raw(), w, h).ok()
}

/// Build and install the tray. Returns the live handle (caller must keep it
/// alive). `None` on failure — we never want the app to crash because a tray
/// couldn't be created.
pub fn install_tray() -> Option<TrayIcon> {
    // No menu: `muda` (which provides menus for tray-icon) registers an ObjC
    // class on macOS, and dioxus-desktop already registers the same class —
    // creating a menu segfaults. Without a menu, the user clicks the icon to
    // bring the window to the front (still the core gesture).
    let icon = icon_image();
    let mut builder = TrayIconBuilder::new()
        .with_tooltip("Kintsugi — guardrails for your AI agents (running). Click to show.");
    if let Some(ico) = icon {
        builder = builder.with_icon(ico);
    }
    let tray = builder.build().ok()?;

    // Background thread that listens for tray clicks and bumps SHOW_REQUESTED.
    std::thread::Builder::new()
        .name("kintsugi-tray".into())
        .spawn(move || {
            let tray_rx = TrayIconEvent::receiver();
            while let Ok(ev) = tray_rx.recv() {
                if matches!(
                    ev,
                    TrayIconEvent::Click {
                        button: tray_icon::MouseButton::Left,
                        button_state: tray_icon::MouseButtonState::Up,
                        ..
                    } | TrayIconEvent::DoubleClick { .. }
                ) {
                    SHOW_REQUESTED.store(true, Ordering::SeqCst);
                }
            }
        })
        .ok();

    Some(tray)
}
