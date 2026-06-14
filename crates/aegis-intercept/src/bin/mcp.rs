//! `aegis-mcp` binary (scaffold).

fn main() -> anyhow::Result<()> {
    println!("aegis-mcp {}", aegis_intercept::VERSION);
    Ok(())
}
