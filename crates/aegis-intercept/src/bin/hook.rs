//! `aegis-hook` binary (scaffold).

fn main() -> anyhow::Result<()> {
    println!("aegis-hook {}", aegis_intercept::VERSION);
    Ok(())
}
