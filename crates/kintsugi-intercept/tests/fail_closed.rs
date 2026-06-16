//! Adversarial: an agent can't re-open the gate by killing the daemon.
//!
//! With the admin fail-closed marker present and the daemon unreachable, the
//! shim must BLOCK a non-catastrophic command — and `KINTSUGI_FAIL_CLOSED=0` in
//! the (agent-controlled) environment must NOT override the marker.
#![cfg(unix)]

use std::os::unix::fs::symlink;
use std::process::Command;

#[test]
fn marker_blocks_when_daemon_is_down_even_with_env_off() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("events.db");
    let dead_sock = tmp.path().join("dead.sock"); // nothing listening
    let shim_dir = tmp.path().join("shimdir");
    std::fs::create_dir_all(&shim_dir).unwrap();

    // The admin marker lives next to the db (what `default_db_path()` resolves
    // from KINTSUGI_DB). Its mere presence means "fail closed".
    std::fs::write(db.with_file_name("fail-closed.flag"), b"fail-closed\n").unwrap();

    // Shim a benign, non-catastrophic command.
    let shim_bin = env!("CARGO_BIN_EXE_kintsugi-shim");
    symlink(shim_bin, shim_dir.join("echo")).unwrap();

    let path = format!(
        "{}:{}",
        shim_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new(shim_dir.join("echo"))
        .arg("hi")
        .env("PATH", &path)
        .env("KINTSUGI_DB", &db)
        .env("KINTSUGI_SOCKET", &dead_sock)
        .env("KINTSUGI_FAIL_CLOSED", "0") // the agent tries to opt OUT
        .output()
        .unwrap();

    assert!(
        !out.status.success(),
        "fail-closed marker must block when the daemon is down, even with env=0"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("fail-closed"),
        "expected a fail-closed message, got: {stderr}"
    );
}

#[test]
fn no_marker_and_env_off_runs_unguarded_when_daemon_down() {
    // Without the marker (and without the env opt-in), the default is fail-open:
    // a benign command still runs when the daemon is down (Kintsugi is a safety
    // net, not a brick — spine #7).
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("events.db");
    let dead_sock = tmp.path().join("dead.sock");
    let shim_dir = tmp.path().join("shimdir");
    std::fs::create_dir_all(&shim_dir).unwrap();

    let shim_bin = env!("CARGO_BIN_EXE_kintsugi-shim");
    symlink(shim_bin, shim_dir.join("echo")).unwrap();
    let path = format!(
        "{}:{}",
        shim_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new(shim_dir.join("echo"))
        .arg("hi")
        .env("PATH", &path)
        .env("KINTSUGI_DB", &db)
        .env("KINTSUGI_SOCKET", &dead_sock)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "default is fail-open for benign commands"
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "hi");
}
