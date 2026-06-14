//! The `aegis` command-line interface.

use clap::{Parser, Subcommand};

/// Aegis — a local-first safety layer for AI coding agents.
#[derive(Debug, Parser)]
#[command(name = "aegis", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show daemon and interception status.
    Status,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None => {
            println!("aegis {}", env!("CARGO_PKG_VERSION"));
            println!("Run `aegis --help` for usage.");
        }
        Some(Command::Status) => {
            println!("aegis {}", env!("CARGO_PKG_VERSION"));
        }
    }
    Ok(())
}
