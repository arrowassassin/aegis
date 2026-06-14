//! P0.4 acceptance: a shimmed `rm` deletes the file AND logs the event, with the
//! real binary's exit code preserved. Unix-only (uses symlinks + a filesystem
//! socket); the same code path covers Windows via named pipes.
#![cfg(unix)]

use std::os::unix::fs::symlink;
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread;

use aegis_core::{Class, Decision, EventLog};
use aegis_daemon::{Daemon, Server};

fn serial_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

struct Harness {
    _guard: MutexGuard<'static, ()>,
    tmp: tempfile::TempDir,
    shim_dir: std::path::PathBuf,
    db: std::path::PathBuf,
    server: Option<thread::JoinHandle<()>>,
}

/// Start a daemon serving `requests` connections, with a shim dir on a private
/// socket/db. Symlink each requested command name to the built `aegis-shim`.
fn start(requests: usize, link_as: &[&str]) -> Harness {
    let guard = serial_lock();
    let tmp = tempfile::tempdir().unwrap();
    let sock = tmp.path().join("aegis.sock");
    let db = tmp.path().join("events.db");
    let shim_dir = tmp.path().join("shimdir");
    std::fs::create_dir_all(&shim_dir).unwrap();

    let shim_bin = env!("CARGO_BIN_EXE_aegis-shim");
    for name in link_as {
        symlink(shim_bin, shim_dir.join(name)).unwrap();
    }

    std::env::set_var("AEGIS_SOCKET", &sock);
    std::env::set_var("AEGIS_DB", &db);

    let db_for_thread = db.clone();
    let server = Server::bind().unwrap();
    let handle = thread::spawn(move || {
        let daemon = Daemon::open(&db_for_thread).unwrap();
        server.serve_n(requests, |cmd| daemon.handle(cmd)).unwrap();
    });

    Harness {
        _guard: guard,
        tmp,
        shim_dir,
        db,
        server: Some(handle),
    }
}

impl Harness {
    /// PATH with the shim dir first, then the inherited PATH.
    fn shimmed_path(&self) -> String {
        let orig = std::env::var("PATH").unwrap_or_default();
        format!("{}:{}", self.shim_dir.display(), orig)
    }

    fn join(&mut self) {
        if let Some(h) = self.server.take() {
            h.join().unwrap();
        }
    }
}

#[test]
fn shimmed_catastrophic_rm_is_held_and_does_not_run() {
    let mut h = start(1, &["rm"]);

    // A directory the real rm -rf would destroy.
    let work = h.tmp.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    let victim = work.join("data");
    std::fs::create_dir_all(&victim).unwrap();
    std::fs::write(victim.join("keep.txt"), b"important").unwrap();

    // The shim has no TTY to approve on, so a held command must NOT run.
    let status = Command::new(h.shim_dir.join("rm"))
        .arg("-rf")
        .arg("data")
        .current_dir(&work)
        .stdin(std::process::Stdio::null())
        .env("PATH", h.shimmed_path())
        .env("AEGIS_SOCKET", h.tmp.path().join("aegis.sock"))
        .env("AEGIS_DB", &h.db)
        .status()
        .unwrap();

    assert!(!status.success(), "a held command must not exit 0");
    assert!(victim.exists(), "the directory must survive — rm was held");

    h.join();

    let log = EventLog::open(&h.db).unwrap();
    let tail = log.tail(10).unwrap();
    assert_eq!(tail.len(), 1);
    assert_eq!(tail[0].agent, "shim");
    assert_eq!(tail[0].command, "rm -rf data");
    assert_eq!(tail[0].class, Class::Catastrophic);
    assert_eq!(tail[0].decision, Decision::Hold);
    assert!(log.verify_chain().unwrap().is_intact());
}

#[test]
fn shimmed_command_propagates_nonzero_exit_code() {
    let mut h = start(1, &["false"]);

    // `false` exits 1; the shim must forward that exact code.
    let status = Command::new(h.shim_dir.join("false"))
        .current_dir(h.tmp.path())
        .env("PATH", h.shimmed_path())
        .env("AEGIS_SOCKET", h.tmp.path().join("aegis.sock"))
        .env("AEGIS_DB", &h.db)
        .status()
        .unwrap();

    assert_eq!(status.code(), Some(1), "exit code must be preserved");

    h.join();
    let log = EventLog::open(&h.db).unwrap();
    assert_eq!(log.tail(10).unwrap()[0].command, "false");
}

#[test]
fn shimmed_command_forwards_stdout() {
    let mut h = start(1, &["echo"]);

    let out = Command::new(h.shim_dir.join("echo"))
        .arg("hello-from-shim")
        .current_dir(h.tmp.path())
        .env("PATH", h.shimmed_path())
        .env("AEGIS_SOCKET", h.tmp.path().join("aegis.sock"))
        .env("AEGIS_DB", &h.db)
        .output()
        .unwrap();

    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "hello-from-shim"
    );

    h.join();
}
