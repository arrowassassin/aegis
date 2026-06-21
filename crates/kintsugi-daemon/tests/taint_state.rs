//! Phase 6 · Item A — the daemon owns session/file taint state.

use kintsugi_core::{SourceKind, TaintEvent, TaintLabel};
use kintsugi_daemon::Daemon;
use time::OffsetDateTime;

fn ingest(session: &str) -> TaintEvent {
    TaintEvent::Ingest {
        label: TaintLabel {
            source_kind: SourceKind::Web,
            source_id: "https://untrusted.example/x".to_string(),
            ts: OffsetDateTime::UNIX_EPOCH,
            agent: "claude-code".to_string(),
            session: session.to_string(),
        },
    }
}

#[test]
fn daemon_tracks_and_resets_session_taint() {
    let tmp = tempfile::tempdir().unwrap();
    let daemon = Daemon::open(tmp.path().join("e.db")).unwrap();

    // Clean to start; an untracked (None) session is never tainted.
    assert!(!daemon.is_session_tainted(Some("s1")));
    assert!(!daemon.is_session_tainted(None));

    // Observing untrusted content taints just that session.
    daemon.apply_taint(&ingest("s1"));
    assert!(daemon.is_session_tainted(Some("s1")));
    assert!(!daemon.is_session_tainted(Some("s2"))); // isolation between sessions

    // A policy-driven reset clears it.
    daemon.apply_taint(&TaintEvent::Reset {
        session: "s1".to_string(),
    });
    assert!(!daemon.is_session_tainted(Some("s1")));
}

#[test]
fn taint_survives_a_daemon_restart() {
    // Item D: taint is persisted to the durable stream and replayed on
    // Daemon::open, so a stop/start (or a watchdog relaunch) does not silently
    // clear it — the trifecta guard stays armed across restarts instead of
    // failing open.
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("e.db");

    {
        let daemon = Daemon::open(db.clone()).unwrap();
        daemon.apply_taint(&ingest("s1"));
        assert!(daemon.is_session_tainted(Some("s1")));
    } // daemon dropped — simulates `kintsugi stop`

    // Cold start against the same data dir replays the durable taint stream.
    let daemon = Daemon::open(db.clone()).unwrap();
    assert!(
        daemon.is_session_tainted(Some("s1")),
        "taint must be rebuilt by replay on open (no fail-open-on-restart)"
    );

    // A persisted Reset is replayed too — taint stays cleared, never resurrected.
    daemon.apply_taint(&TaintEvent::Reset {
        session: "s1".to_string(),
    });
    drop(daemon);
    let daemon = Daemon::open(db).unwrap();
    assert!(
        !daemon.is_session_tainted(Some("s1")),
        "a replayed Reset keeps the session clean across a restart"
    );
}
