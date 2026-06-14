# Decision log

One line per notable build decision, newest last. See `aegis-design-doc.md` for
the locked product decisions this build implements.

- P0.1: Rust workspace with six crates (`aegis-core`, `aegis-daemon`,
  `aegis-intercept`, `aegis-cli`, `aegis-model`, `aegis-tui`). Edition 2021,
  resolver 2. `aegis` binary prints its version. IPC will use the `interprocess`
  crate for portable local sockets / named pipes.
