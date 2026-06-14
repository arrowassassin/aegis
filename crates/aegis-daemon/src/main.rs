//! `aegis-daemon` binary entry point.

fn main() -> anyhow::Result<()> {
    println!("aegis-daemon {}", aegis_daemon::VERSION);
    Ok(())
}
