//! Daemon-side authenticated shutdown (closes the CLI-gate env-var bypass).
//!
//! The daemon enforces the admin password against the vault IT loaded at startup,
//! via a challenge-response — the caller's environment can't redirect the check.
#![cfg(unix)]

use std::sync::{Mutex, MutexGuard, OnceLock};

use kintsugi_core::admin;
use kintsugi_daemon::{ipc, Daemon};

fn serial_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Provision a vault at `path` and point the daemon's resolver at it.
fn provision_at(path: &std::path::Path, pw: &str) {
    let prov = admin::provision(pw, &admin::LockedSettings::default()).unwrap();
    admin::save_vault(path, &prov.vault).unwrap();
}

fn challenge(resp: ipc::Response) -> (bool, String, String, admin::KdfParams) {
    match resp {
        ipc::Response::Challenge {
            locked,
            nonce,
            salt,
            params,
        } => (locked, nonce, salt, params),
        other => panic!("expected Challenge, got {other:?}"),
    }
}

#[test]
fn locked_daemon_requires_a_valid_proof_to_stop() {
    let _g = serial_lock();
    let tmp = tempfile::tempdir().unwrap();
    let vault = tmp.path().join("vault.json");
    let db = tmp.path().join("events.db");
    provision_at(&vault, "correct horse battery");
    std::env::set_var("KINTSUGI_VAULT", &vault);
    std::env::set_var("KINTSUGI_DB", &db);

    let daemon = Daemon::open(&db).unwrap();

    // Begin: the daemon reports it is locked and hands out a challenge.
    let (locked, nonce, salt, params) = challenge(daemon.handle_request(ipc::Request::AuthBegin {
        op: "shutdown".into(),
    }));
    assert!(locked, "a provisioned daemon must be locked");

    // A wrong password yields a proof the daemon rejects; it does NOT shut down.
    let nonce_bytes = hex::decode(&nonce).unwrap();
    let bad = admin::compute_proof("guess", &salt, params, &nonce_bytes, b"shutdown").unwrap();
    let resp = daemon.handle_request(ipc::Request::Shutdown {
        op: "shutdown".into(),
        nonce: Some(nonce.clone()),
        proof: Some(hex::encode(bad)),
    });
    assert!(matches!(resp, ipc::Response::Error { .. }));
    assert!(!daemon.should_shutdown(), "wrong password must not stop it");

    // Re-begin (challenge is one-shot), then the correct proof stops it.
    let (_locked, nonce, salt, params) =
        challenge(daemon.handle_request(ipc::Request::AuthBegin {
            op: "shutdown".into(),
        }));
    let nonce_bytes = hex::decode(&nonce).unwrap();
    let good = admin::compute_proof(
        "correct horse battery",
        &salt,
        params,
        &nonce_bytes,
        b"shutdown",
    )
    .unwrap();
    let resp = daemon.handle_request(ipc::Request::Shutdown {
        op: "shutdown".into(),
        nonce: Some(nonce),
        proof: Some(hex::encode(good)),
    });
    assert!(matches!(resp, ipc::Response::Ack));
    assert!(daemon.should_shutdown(), "correct password stops it");

    std::env::remove_var("KINTSUGI_VAULT");
    std::env::remove_var("KINTSUGI_DB");
}

#[test]
fn a_captured_proof_cannot_be_replayed() {
    let _g = serial_lock();
    let tmp = tempfile::tempdir().unwrap();
    let vault = tmp.path().join("vault.json");
    let db = tmp.path().join("events.db");
    provision_at(&vault, "correct horse battery");
    std::env::set_var("KINTSUGI_VAULT", &vault);
    std::env::set_var("KINTSUGI_DB", &db);

    let daemon = Daemon::open(&db).unwrap();
    let (_l, nonce, salt, params) = challenge(daemon.handle_request(ipc::Request::AuthBegin {
        op: "shutdown".into(),
    }));
    let nonce_bytes = hex::decode(&nonce).unwrap();
    let proof = hex::encode(
        admin::compute_proof(
            "correct horse battery",
            &salt,
            params,
            &nonce_bytes,
            b"shutdown",
        )
        .unwrap(),
    );

    // First use succeeds.
    assert!(matches!(
        daemon.handle_request(ipc::Request::Shutdown {
            op: "shutdown".into(),
            nonce: Some(nonce.clone()),
            proof: Some(proof.clone()),
        }),
        ipc::Response::Ack
    ));

    // Replaying the SAME proof after the one-shot challenge is consumed → rejected.
    let resp = daemon.handle_request(ipc::Request::Shutdown {
        op: "shutdown".into(),
        nonce: Some(nonce),
        proof: Some(proof),
    });
    assert!(
        matches!(resp, ipc::Response::Error { .. }),
        "replay must fail"
    );

    std::env::remove_var("KINTSUGI_VAULT");
    std::env::remove_var("KINTSUGI_DB");
}

#[test]
fn repeated_failures_lock_out_brute_force() {
    let _g = serial_lock();
    let tmp = tempfile::tempdir().unwrap();
    let vault = tmp.path().join("vault.json");
    let db = tmp.path().join("events.db");
    provision_at(&vault, "correct horse battery");
    std::env::set_var("KINTSUGI_VAULT", &vault);
    std::env::set_var("KINTSUGI_DB", &db);

    let daemon = Daemon::open(&db).unwrap();
    // Hammer wrong proofs; past the free budget the daemon locks out and refuses
    // to even check the proof, returning a "locked out" message.
    let mut locked_out = false;
    for _ in 0..8 {
        if let ipc::Response::Error { message } = daemon.handle_request(ipc::Request::Shutdown {
            op: "shutdown".into(),
            nonce: None,
            proof: Some("00".into()),
        }) {
            if message.contains("locked out") {
                locked_out = true;
                break;
            }
        }
    }
    assert!(locked_out, "a brute-force run must trigger a lockout");
    assert!(!daemon.should_shutdown(), "bad auth never stops the daemon");

    std::env::remove_var("KINTSUGI_VAULT");
    std::env::remove_var("KINTSUGI_DB");
}

#[test]
fn a_vault_provisioned_after_startup_is_honored_without_a_restart() {
    // The trap that bit in practice: the daemon used to read the vault only at
    // startup, so a `kintsugi admin provision` after the fact silently didn't apply
    // until a restart. The daemon now reads the vault fresh per op.
    let _g = serial_lock();
    let tmp = tempfile::tempdir().unwrap();
    let vault = tmp.path().join("vault.json");
    let db = tmp.path().join("events.db");
    std::env::set_var("KINTSUGI_VAULT", &vault);
    std::env::set_var("KINTSUGI_DB", &db);

    // Daemon starts with NO vault (unprovisioned).
    let daemon = Daemon::open(&db).unwrap();

    // Provision AFTER startup — no restart.
    provision_at(&vault, "correct horse battery");

    // The daemon now reports locked (it re-read the vault) and the correct proof
    // stops it — proving it authenticates against the freshly provisioned vault.
    let (locked, nonce, salt, params) = challenge(daemon.handle_request(ipc::Request::AuthBegin {
        op: "shutdown".into(),
    }));
    assert!(
        locked,
        "a vault provisioned after startup must lock the daemon"
    );
    let nonce_bytes = hex::decode(&nonce).unwrap();
    let good = admin::compute_proof(
        "correct horse battery",
        &salt,
        params,
        &nonce_bytes,
        b"shutdown",
    )
    .unwrap();
    let resp = daemon.handle_request(ipc::Request::Shutdown {
        op: "shutdown".into(),
        nonce: Some(nonce),
        proof: Some(hex::encode(good)),
    });
    assert!(matches!(resp, ipc::Response::Ack));
    assert!(daemon.should_shutdown());

    std::env::remove_var("KINTSUGI_VAULT");
    std::env::remove_var("KINTSUGI_DB");
}

#[test]
fn a_reprovision_rotates_credentials_live_without_a_restart() {
    // Re-provisioning (e.g. to fix a vault that predated the proof scheme) takes
    // effect on the running daemon: the old password stops working, the new one
    // works — no restart, no stale-vault confusion.
    let _g = serial_lock();
    let tmp = tempfile::tempdir().unwrap();
    let vault = tmp.path().join("vault.json");
    let db = tmp.path().join("events.db");
    provision_at(&vault, "old password one");
    std::env::set_var("KINTSUGI_VAULT", &vault);
    std::env::set_var("KINTSUGI_DB", &db);

    let daemon = Daemon::open(&db).unwrap();

    // Rotate to a new password while the daemon runs.
    provision_at(&vault, "new password two");

    // The OLD password no longer authenticates (verified against the new vault).
    let (_l, nonce, salt, params) = challenge(daemon.handle_request(ipc::Request::AuthBegin {
        op: "shutdown".into(),
    }));
    let nb = hex::decode(&nonce).unwrap();
    let old = admin::compute_proof("old password one", &salt, params, &nb, b"shutdown").unwrap();
    let resp = daemon.handle_request(ipc::Request::Shutdown {
        op: "shutdown".into(),
        nonce: Some(nonce.clone()),
        proof: Some(hex::encode(old)),
    });
    assert!(
        matches!(resp, ipc::Response::Error { .. }),
        "old password must fail"
    );
    assert!(!daemon.should_shutdown());

    // The NEW password does — live, no restart.
    let (_l, nonce, salt, params) = challenge(daemon.handle_request(ipc::Request::AuthBegin {
        op: "shutdown".into(),
    }));
    let nb = hex::decode(&nonce).unwrap();
    let new = admin::compute_proof("new password two", &salt, params, &nb, b"shutdown").unwrap();
    let resp = daemon.handle_request(ipc::Request::Shutdown {
        op: "shutdown".into(),
        nonce: Some(nonce),
        proof: Some(hex::encode(new)),
    });
    assert!(
        matches!(resp, ipc::Response::Ack),
        "new password must stop it"
    );
    assert!(daemon.should_shutdown());

    std::env::remove_var("KINTSUGI_VAULT");
    std::env::remove_var("KINTSUGI_DB");
}

#[test]
fn unprovisioned_daemon_stops_without_a_password() {
    let _g = serial_lock();
    let tmp = tempfile::tempdir().unwrap();
    let vault = tmp.path().join("absent.json"); // never created → unprovisioned
    let db = tmp.path().join("events.db");
    std::env::set_var("KINTSUGI_VAULT", &vault);
    std::env::set_var("KINTSUGI_DB", &db);

    let daemon = Daemon::open(&db).unwrap();
    let (locked, _n, _s, _p) = challenge(daemon.handle_request(ipc::Request::AuthBegin {
        op: "shutdown".into(),
    }));
    assert!(!locked, "no vault → not locked");
    let resp = daemon.handle_request(ipc::Request::Shutdown {
        op: "shutdown".into(),
        nonce: None,
        proof: None,
    });
    assert!(matches!(resp, ipc::Response::Ack));
    assert!(daemon.should_shutdown());

    std::env::remove_var("KINTSUGI_VAULT");
    std::env::remove_var("KINTSUGI_DB");
}
