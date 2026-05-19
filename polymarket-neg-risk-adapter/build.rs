use anyhow::Result;
use std::fs;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("NegRiskAdapter", "abi/NegRiskAdapter.json")?
        .generate()?
        .write_to_file("src/abi/neg_risk_adapter.rs")?;

    let generated = fs::read_to_string("src/abi/neg_risk_adapter.rs")?;
    fs::write(
        "src/abi/neg_risk_adapter.rs",
        format!("#![allow(clippy::all)]\n{generated}"),
    )?;

    Ok(())
}
