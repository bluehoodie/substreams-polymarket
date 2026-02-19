use anyhow::Result;
use std::fs;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("CTFExchange", "abi/CTFExchange_events_only.json")?
        .generate()?
        .write_to_file("src/abi/ctf_exchange.rs")?;

    let generated = fs::read_to_string("src/abi/ctf_exchange.rs")?;
    fs::write(
        "src/abi/ctf_exchange.rs",
        format!("#![allow(clippy::all)]\n{generated}"),
    )?;

    Ok(())
}
