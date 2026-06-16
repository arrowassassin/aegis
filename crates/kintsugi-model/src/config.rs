//! Persisted model selection.
//!
//! Which GGUF the daemon should load is stored in a one-line file in the data
//! dir, so the choice survives a daemon restart without depending on a shell
//! env var (`KINTSUGI_MODEL_FILE`). `kintsugi model use` writes it; the daemon's
//! `LlamaScorer::autoload` reads it after the env override. This is the
//! bring-your-own-weights path: any GGUF works, so a model can be swapped at any
//! time without updating Kintsugi itself.
//!
//! Always compiled (not behind `llama`) so the CLI can manage the selection even
//! when the installed daemon has no inference engine.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// The data dir Kintsugi uses for the event log, socket, and this config.
/// `KINTSUGI_DATA_DIR` overrides the platform default (and keeps tests
/// deterministic); it must match the daemon's resolution so both read one file.
fn data_dir() -> Option<PathBuf> {
    if let Ok(d) = std::env::var("KINTSUGI_DATA_DIR") {
        return Some(PathBuf::from(d));
    }
    directories::ProjectDirs::from("", "", "kintsugi").map(|p| p.data_dir().to_path_buf())
}

/// The file recording the configured model path (one line: an absolute path).
pub fn model_config_path() -> Option<PathBuf> {
    data_dir().map(|d| d.join("model.path"))
}

/// The persisted model path, if one is recorded. Returns the path even when the
/// file no longer exists on disk, so callers can report a stale selection rather
/// than silently ignore it.
pub fn configured_model() -> Option<PathBuf> {
    let cfg = model_config_path()?;
    let raw = std::fs::read_to_string(&cfg).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(PathBuf::from(trimmed))
}

/// Persist `path` as the configured model, creating the data dir if needed.
pub fn set_configured_model(path: &Path) -> Result<()> {
    let cfg = model_config_path().context("could not resolve the Kintsugi data dir")?;
    if let Some(parent) = cfg.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    std::fs::write(&cfg, format!("{}\n", path.display()))
        .with_context(|| format!("write {}", cfg.display()))?;
    Ok(())
}

/// Forget the persisted selection. A no-op if none is set.
pub fn clear_configured_model() -> Result<()> {
    if let Some(cfg) = model_config_path() {
        if cfg.exists() {
            std::fs::remove_file(&cfg).with_context(|| format!("remove {}", cfg.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A guard that points the data dir at a temp dir for the duration of a test.
    /// `KINTSUGI_DATA_DIR` is process-global, so the tests run serially under a
    /// lock to avoid cross-test interference.
    fn with_data_dir<T>(f: impl FnOnce(&Path) -> T) -> T {
        use std::sync::{Mutex, OnceLock};
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _g = LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("KINTSUGI_DATA_DIR", tmp.path());
        let out = f(tmp.path());
        std::env::remove_var("KINTSUGI_DATA_DIR");
        out
    }

    #[test]
    fn round_trips_a_selection() {
        with_data_dir(|_| {
            assert!(configured_model().is_none(), "starts empty");
            let model = PathBuf::from("/models/qwen.gguf");
            set_configured_model(&model).unwrap();
            assert_eq!(configured_model(), Some(model));
        });
    }

    #[test]
    fn clear_removes_the_selection() {
        with_data_dir(|_| {
            set_configured_model(Path::new("/models/a.gguf")).unwrap();
            clear_configured_model().unwrap();
            assert!(configured_model().is_none());
            // Clearing again is a no-op, not an error.
            clear_configured_model().unwrap();
        });
    }

    #[test]
    fn blank_or_whitespace_file_reads_as_unset() {
        with_data_dir(|_| {
            let cfg = model_config_path().unwrap();
            std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();
            std::fs::write(&cfg, "   \n").unwrap();
            assert!(configured_model().is_none());
        });
    }

    #[test]
    fn set_creates_the_data_dir() {
        with_data_dir(|root| {
            // Point at a not-yet-created nested dir to exercise create_dir_all.
            let nested = root.join("nested/deeper");
            std::env::set_var("KINTSUGI_DATA_DIR", &nested);
            set_configured_model(Path::new("/m/x.gguf")).unwrap();
            assert!(nested.join("model.path").is_file());
        });
    }
}
