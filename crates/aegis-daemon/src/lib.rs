//! Aegis resident daemon library.
//!
//! Long-lived process that owns the event log and runs the decision loop. The
//! interception layer connects over a local socket, sends a `ProposedCommand`,
//! and blocks on the returned `Verdict`.

#![forbid(unsafe_code)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
