//! Cross-platform self-install for the desktop binary.
//!
//! Registers the running binary as a desktop app on the host OS, using icons
//! baked into the binary (no external files needed). The Rust port of
//! `install.sh`, so `cargo install kintsugi-control-room && kintsugi-control-room --install`
//! is a complete OS-integrated install path.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

const APP_NAME: &str = "Kintsugi";
const BIN_NAME: &str = "kintsugi-control-room";
const BUNDLE_ID: &str = "tools.kintsugi.control-room";

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
}

fn current_exe() -> Result<PathBuf> {
    std::env::current_exe().context("could not resolve current exe")
}

/// All PNG sizes baked into the binary, returned alongside the size in pixels.
fn icon_pngs() -> Vec<(u32, &'static [u8])> {
    vec![
        (16, crate::ICON_PNG_16),
        (32, crate::ICON_PNG_32),
        (64, crate::ICON_PNG_64),
        (128, crate::ICON_PNG_128),
        (256, crate::ICON_PNG),
        (512, crate::ICON_PNG_512),
    ]
}

// ---- macOS ----------------------------------------------------------------

#[cfg(target_os = "macos")]
fn app_bundle_path() -> PathBuf {
    home().join("Applications").join(format!("{APP_NAME}.app"))
}

#[cfg(target_os = "macos")]
fn install_macos() -> Result<PathBuf> {
    let app = app_bundle_path();
    let _ = std::fs::remove_dir_all(&app);
    let contents = app.join("Contents");
    let macos = contents.join("MacOS");
    let res = contents.join("Resources");
    std::fs::create_dir_all(&macos)?;
    std::fs::create_dir_all(&res)?;

    // Write per-size PNGs into a temp .iconset, then convert with iconutil.
    let tmp = std::env::temp_dir().join(format!("{APP_NAME}-{}.iconset", std::process::id()));
    std::fs::create_dir_all(&tmp)?;
    for (size, bytes) in icon_pngs() {
        std::fs::write(tmp.join(format!("icon_{size}x{size}.png")), bytes)?;
    }
    // Retina @2x using the next-larger PNG.
    let pairs = [(16u32, 32u32), (32, 64), (128, 256), (256, 512)];
    for (s, src_size) in pairs {
        if let Some((_, bytes)) = icon_pngs().into_iter().find(|(sz, _)| *sz == src_size) {
            std::fs::write(tmp.join(format!("icon_{s}x{s}@2x.png")), bytes)?;
        }
    }

    let icns = res.join(format!("{APP_NAME}.icns"));
    let status = std::process::Command::new("iconutil")
        .args(["-c", "icns"])
        .arg(&tmp)
        .arg("-o")
        .arg(&icns)
        .status()
        .context("iconutil")?;
    anyhow::ensure!(status.success(), "iconutil failed");
    let _ = std::fs::remove_dir_all(&tmp);

    // Drop a real binary in MacOS/ so Launch Services treats it as a proper app.
    let src = current_exe()?;
    let dest = macos.join(APP_NAME);
    std::fs::copy(&src, &dest)?;
    // Make sure it's executable.
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(&dest)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&dest, perms)?;

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>{APP_NAME}</string>
  <key>CFBundleDisplayName</key><string>{APP_NAME}</string>
  <key>CFBundleExecutable</key><string>{APP_NAME}</string>
  <key>CFBundleIdentifier</key><string>{BUNDLE_ID}</string>
  <key>CFBundleVersion</key><string>0.2.1</string>
  <key>CFBundleShortVersionString</key><string>0.2.1</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleIconFile</key><string>{APP_NAME}</string>
  <key>LSMinimumSystemVersion</key><string>10.13</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
"#
    );
    std::fs::write(contents.join("Info.plist"), plist)?;
    Ok(app)
}

#[cfg(target_os = "macos")]
fn uninstall_macos() -> Result<()> {
    let app = app_bundle_path();
    if app.exists() {
        std::fs::remove_dir_all(&app).with_context(|| format!("remove {}", app.display()))?;
    }
    Ok(())
}

// ---- Linux ----------------------------------------------------------------

#[cfg(target_os = "linux")]
fn install_linux() -> Result<PathBuf> {
    let bin_dest = home().join(".local/bin").join(BIN_NAME);
    std::fs::create_dir_all(bin_dest.parent().unwrap())?;
    // Copy this binary to a stable PATH location.
    std::fs::copy(current_exe()?, &bin_dest)?;
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(&bin_dest)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&bin_dest, perms)?;

    // Hicolor icon set.
    let icon_root = home().join(".local/share/icons/hicolor");
    for (size, bytes) in icon_pngs() {
        let dir = icon_root.join(format!("{size}x{size}/apps"));
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join(format!("{BIN_NAME}.png")), bytes)?;
    }

    // .desktop entry.
    let apps = home().join(".local/share/applications");
    std::fs::create_dir_all(&apps)?;
    let desktop = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name={APP_NAME}\n\
         Comment=Local-first command governance for AI coding agents\n\
         Exec={}\n\
         Icon={BIN_NAME}\n\
         Terminal=false\n\
         Categories=Utility;Security;\n\
         StartupWMClass={BIN_NAME}\n",
        bin_dest.display()
    );
    let desktop_path = apps.join(format!("{BIN_NAME}.desktop"));
    std::fs::write(&desktop_path, desktop)?;

    // Best-effort cache refresh — these are no-ops if the tools aren't installed.
    let _ = std::process::Command::new("update-desktop-database").arg(&apps).status();
    let _ = std::process::Command::new("gtk-update-icon-cache").arg(&icon_root).status();
    Ok(desktop_path)
}

#[cfg(target_os = "linux")]
fn uninstall_linux() -> Result<()> {
    let _ = std::fs::remove_file(home().join(".local/bin").join(BIN_NAME));
    let _ = std::fs::remove_file(
        home().join(".local/share/applications").join(format!("{BIN_NAME}.desktop")),
    );
    for (size, _) in icon_pngs() {
        let _ = std::fs::remove_file(
            home()
                .join(".local/share/icons/hicolor")
                .join(format!("{size}x{size}/apps/{BIN_NAME}.png")),
        );
    }
    Ok(())
}

// ---- Windows --------------------------------------------------------------

#[cfg(target_os = "windows")]
fn install_windows() -> Result<PathBuf> {
    // Drop the binary + a Start-menu shortcut. ICO generation from PNGs done in
    // Rust (the `ico` crate would be ideal; for now write an ICO manually using
    // the 256-px PNG as PNG-encoded ICO entry — Windows accepts that).
    let local_app = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join("AppData/Local"));
    let dest_dir = local_app.join("Programs").join(APP_NAME);
    std::fs::create_dir_all(&dest_dir)?;
    let dest_bin = dest_dir.join(format!("{APP_NAME}.exe"));
    std::fs::copy(current_exe()?, &dest_bin)?;

    let ico_path = dest_dir.join("Kintsugi.ico");
    write_png_in_ico(&ico_path, crate::ICON_PNG)?;

    // Start-menu shortcut (.lnk) — generate via PowerShell so we don't need
    // extra crates. Best-effort; if it fails, the binary still works.
    let start_menu = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join("AppData/Roaming"))
        .join("Microsoft/Windows/Start Menu/Programs");
    std::fs::create_dir_all(&start_menu)?;
    let lnk = start_menu.join(format!("{APP_NAME}.lnk"));
    let ps = format!(
        "$s = (New-Object -COM WScript.Shell).CreateShortcut('{}'); \
         $s.TargetPath = '{}'; $s.IconLocation = '{}'; $s.Save();",
        lnk.display().to_string().replace('\'', "''"),
        dest_bin.display().to_string().replace('\'', "''"),
        ico_path.display().to_string().replace('\'', "''"),
    );
    let _ = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps])
        .status();
    Ok(dest_bin)
}

#[cfg(target_os = "windows")]
fn uninstall_windows() -> Result<()> {
    let local_app = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join("AppData/Local"));
    let dir = local_app.join("Programs").join(APP_NAME);
    let _ = std::fs::remove_dir_all(&dir);
    let start_menu = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join("AppData/Roaming"))
        .join("Microsoft/Windows/Start Menu/Programs");
    let _ = std::fs::remove_file(start_menu.join(format!("{APP_NAME}.lnk")));
    Ok(())
}

/// Wrap a PNG inside a minimal ICO container. Windows accepts PNG-encoded
/// ICONDIR entries for sizes ≥ 32x32, so this is a one-icon ICO.
#[cfg(target_os = "windows")]
fn write_png_in_ico(path: &Path, png: &[u8]) -> Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;
    // ICONDIR
    f.write_all(&[0, 0])?; // reserved
    f.write_all(&[1, 0])?; // type = icon
    f.write_all(&[1, 0])?; // count = 1
    // ICONDIRENTRY: 256x256
    f.write_all(&[0, 0])?; // width 0 = 256
    f.write_all(&[0, 0])?; // height 0 = 256
    f.write_all(&[0])?;    // 0 colors
    f.write_all(&[0])?;    // reserved
    f.write_all(&1u16.to_le_bytes())?; // planes
    f.write_all(&32u16.to_le_bytes())?; // bpp
    f.write_all(&(png.len() as u32).to_le_bytes())?; // size
    f.write_all(&22u32.to_le_bytes())?; // offset
    f.write_all(png)?;
    Ok(())
}

// ---- Public surface --------------------------------------------------------

pub fn install_app() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let app = install_macos()?;
        println!("✓ installed {}", app.display());
        println!("  Open from Launchpad/Spotlight, or:  open '{}'", app.display());
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        let dotdesktop = install_linux()?;
        println!("✓ installed {}", dotdesktop.display());
        println!("  Should appear in your launcher.");
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    {
        let bin = install_windows()?;
        println!("✓ installed {}", bin.display());
        println!("  Search for 'Kintsugi' in the Start menu.");
        return Ok(());
    }
    #[allow(unreachable_code)]
    {
        anyhow::bail!("unsupported OS — run the binary directly")
    }
}

pub fn uninstall_app() -> Result<()> {
    #[cfg(target_os = "macos")]
    { uninstall_macos()?; println!("✓ uninstalled the Kintsugi.app bundle"); return Ok(()); }
    #[cfg(target_os = "linux")]
    { uninstall_linux()?; println!("✓ removed the Linux desktop entry + icons"); return Ok(()); }
    #[cfg(target_os = "windows")]
    { uninstall_windows()?; println!("✓ removed the Windows program dir + Start-menu shortcut"); return Ok(()); }
    #[allow(unreachable_code)]
    { anyhow::bail!("unsupported OS") }
}
