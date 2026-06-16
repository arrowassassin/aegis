//! `kintsugi admin` — provision/manage the password-locked settings, and the
//! gate that makes stopping Kintsugi require the admin password.
//!
//! Honest scope (see the design doc threat matrix): this defeats an AI agent or a
//! normal user running `kintsugi stop` / `kintsugi admin …` without the password —
//! the 90% case. It does **not** stop a root user, who can `kill` the daemon PID
//! directly; the watchdog (relaunch) and daemon-side IPC enforcement are the
//! stronger follow-on tiers. We make a forced stop harder and (later) visible,
//! never claim it's impossible.

use std::io::{Read, Write};
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use kintsugi_core::admin::{self, LockedSettings, VaultState};

/// Where the sealed admin vault lives. Overridable via `KINTSUGI_VAULT` (tests /
/// a root-owned `/etc/kintsugi/` path in the locked system posture).
pub fn vault_path() -> PathBuf {
    if let Ok(p) = std::env::var("KINTSUGI_VAULT") {
        return PathBuf::from(p);
    }
    if let Some(dirs) = directories::ProjectDirs::from("", "", "kintsugi") {
        return dirs.data_dir().join("admin-vault.json");
    }
    std::env::temp_dir().join("kintsugi-admin-vault.json")
}

const MIN_PASSWORD_LEN: usize = 8;

/// `kintsugi admin provision` — set the admin password and lock the settings.
pub fn provision(password_file: Option<PathBuf>, force: bool) -> Result<()> {
    let path = vault_path();
    if let VaultState::Locked(_) = admin::load_vault(&path) {
        if !force {
            bail!(
                "already provisioned at {}\n  Use --force to re-provision (rotates the password and recovery key).",
                path.display()
            );
        }
    }
    let pw = read_password("Set admin password: ", &password_file)?;
    if pw.chars().count() < MIN_PASSWORD_LEN {
        bail!("password too short (minimum {MIN_PASSWORD_LEN} characters)");
    }
    if password_file.is_none() {
        let confirm = read_password_tty("Confirm admin password: ")?;
        if pw != confirm {
            bail!("passwords did not match");
        }
    }
    let prov =
        admin::provision(&pw, &LockedSettings::default()).map_err(|e| anyhow::anyhow!("{e}"))?;
    admin::save_vault(&path, &prov.vault)
        .with_context(|| format!("write vault {}", path.display()))?;

    println!("✓ Kintsugi is now admin-locked — stopping or disabling it requires this password.");
    println!("  vault: {}", path.display());
    println!();
    println!("  RECOVERY KEY — store this offline. It is shown ONCE and cannot be");
    println!("  recovered. It can unlock the settings if the password is lost:");
    println!();
    println!("    {}", prov.recovery_key);
    println!();
    Ok(())
}

/// `kintsugi admin status` — show the lock state (no password needed).
pub fn status() -> Result<()> {
    match admin::load_vault(&vault_path()) {
        VaultState::Unprovisioned => {
            println!("admin lock: not provisioned (unlocked)");
            println!("  Run `kintsugi admin provision` to lock settings behind a password.");
        }
        VaultState::Locked(_) => {
            println!("admin lock: LOCKED");
            println!("  Stopping / disabling Kintsugi requires the admin password.");
        }
        VaultState::Degraded(reason) => {
            println!("admin lock: DEGRADED — {reason}");
            println!("  Privileged operations are refused until the vault is restored or");
            println!("  you re-provision with `--force` (using the recovery key offline).");
        }
    }
    Ok(())
}

/// `kintsugi admin change-password`.
pub fn change_password() -> Result<()> {
    let path = vault_path();
    let vault = match admin::load_vault(&path) {
        VaultState::Locked(v) => *v,
        VaultState::Unprovisioned => bail!("not provisioned — nothing to change"),
        VaultState::Degraded(r) => bail!("vault is degraded ({r}); restore or re-provision first"),
    };
    let old = read_password_tty("Current admin password: ")?;
    let new = read_password_tty("New admin password: ")?;
    if new.chars().count() < MIN_PASSWORD_LEN {
        bail!("password too short (minimum {MIN_PASSWORD_LEN} characters)");
    }
    let confirm = read_password_tty("Confirm new password: ")?;
    if new != confirm {
        bail!("new passwords did not match");
    }
    let prov = vault
        .change_password(&old, &new)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    admin::save_vault(&path, &prov.vault)?;
    println!("✓ Admin password changed.");
    println!();
    println!("  NEW RECOVERY KEY (the previous one no longer works):");
    println!();
    println!("    {}", prov.recovery_key);
    println!();
    Ok(())
}

/// Whether `kintsugi stop` is allowed to proceed. Unprovisioned → yes; Locked →
/// only with the correct password; Degraded → refuse (fail-closed).
pub fn allow_stop() -> bool {
    match admin::load_vault(&vault_path()) {
        VaultState::Unprovisioned => true,
        VaultState::Degraded(reason) => {
            eprintln!(
                "kintsugi: admin vault is degraded ({reason}); refusing to stop.\n  \
                 Restore the vault, or re-provision with the recovery key."
            );
            false
        }
        VaultState::Locked(vault) => match read_password_tty("Admin password to stop Kintsugi: ") {
            Ok(pw) if vault.verify_password(&pw) => true,
            Ok(_) => {
                eprintln!("kintsugi: wrong admin password — not stopping.");
                false
            }
            Err(e) => {
                eprintln!("kintsugi: {e}");
                false
            }
        },
    }
}

/// Read a password from a file (trailing newline trimmed) or interactively.
fn read_password(prompt: &str, file: &Option<PathBuf>) -> Result<String> {
    if let Some(f) = file {
        let s = std::fs::read_to_string(f)
            .with_context(|| format!("read password file {}", f.display()))?;
        return Ok(s.trim_end_matches(['\n', '\r']).to_string());
    }
    read_password_tty(prompt)
}

/// Read a line from the real terminal with echo disabled. Reads `/dev/tty`, not
/// stdin, so an agent with piped stdio can't feed the password and a recorder
/// can't capture it from the command line.
fn read_password_tty(prompt: &str) -> Result<String> {
    let mut tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .context("no terminal for password entry — use --password-file")?;
    write!(tty, "{prompt}")?;
    tty.flush()?;
    set_echo(false);
    let mut buf = [0u8; 512];
    let n = tty.read(&mut buf).unwrap_or(0);
    set_echo(true);
    let _ = writeln!(tty);
    let line = String::from_utf8_lossy(&buf[..n]);
    Ok(line.trim_end_matches(['\n', '\r']).to_string())
}

/// Toggle terminal echo on the controlling tty (so the password isn't shown).
#[cfg(unix)]
fn set_echo(on: bool) {
    if let Ok(tty) = std::fs::File::open("/dev/tty") {
        let _ = std::process::Command::new("stty")
            .arg(if on { "echo" } else { "-echo" })
            .stdin(tty)
            .status();
    }
}
#[cfg(not(unix))]
fn set_echo(_on: bool) {}
