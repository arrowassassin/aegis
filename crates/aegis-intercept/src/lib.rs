//! Aegis interception adapters.
//!
//! Three sources, one normalized event. Each adapter turns an agent's proposed
//! command into a [`aegis_core`]-defined `ProposedCommand`, sends it to the
//! daemon, and enforces the returned `Verdict`.

#![forbid(unsafe_code)]

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
