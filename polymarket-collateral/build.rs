use anyhow::Result;
use std::fs;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("PUsd", "abi/pUSD.json")?
        .generate()?
        .write_to_file("src/abi/p_usd.rs")?;
    let generated = fs::read_to_string("src/abi/p_usd.rs")?;
    fs::write("src/abi/p_usd.rs", format!("#![allow(clippy::all)]\n{generated}"))?;

    Abigen::new("CtfCollateralAdapter", "abi/CtfCollateralAdapter.json")?
        .generate()?
        .write_to_file("src/abi/ctf_collateral_adapter.rs")?;
    let generated = fs::read_to_string("src/abi/ctf_collateral_adapter.rs")?;
    fs::write("src/abi/ctf_collateral_adapter.rs", format!("#![allow(clippy::all)]\n{generated}"))?;

    Abigen::new("NegRiskCtfCollateralAdapter", "abi/NegRiskCtfCollateralAdapter.json")?
        .generate()?
        .write_to_file("src/abi/neg_risk_ctf_collateral_adapter.rs")?;
    let generated = fs::read_to_string("src/abi/neg_risk_ctf_collateral_adapter.rs")?;
    fs::write("src/abi/neg_risk_ctf_collateral_adapter.rs", format!("#![allow(clippy::all)]\n{generated}"))?;

    Ok(())
}
