//! `aegis-shim` binary (scaffold).

fn main() -> anyhow::Result<()> {
    println!("aegis-shim {}", aegis_intercept::VERSION);
    Ok(())
}
