use anyhow::Result;
use std::fs;
use substreams_ethereum::Abigen;

fn gen(name: &str, abi: &str, out: &str) -> Result<()> {
    Abigen::new(name, abi)?.generate()?.write_to_file(out)?;
    let generated = fs::read_to_string(out)?;
    fs::write(out, format!("#![allow(clippy::all)]\n{generated}"))?;
    Ok(())
}

fn main() -> Result<()> {
    gen("ConditionalTokens", "abi/ConditionalTokens.json", "src/abi/conditional_tokens.rs")?;
    gen("CtfExchange", "abi/CTFExchange_events_only.json", "src/abi/ctf_exchange.rs")?;
    gen("NegRiskCtf", "abi/NegRiskCTF.json", "src/abi/neg_risk_ctf.rs")?;
    gen("NegRiskAdapter", "abi/NegRiskAdapter.json", "src/abi/neg_risk_adapter.rs")?;
    gen("PUsd", "abi/pUSD.json", "src/abi/p_usd.rs")?;
    gen("CtfCollateralAdapter", "abi/CtfCollateralAdapter.json", "src/abi/ctf_collateral_adapter.rs")?;
    gen("NegRiskCtfCollateralAdapter", "abi/NegRiskCtfCollateralAdapter.json", "src/abi/neg_risk_ctf_collateral_adapter.rs")?;
    gen("DepositWalletFactory", "abi/DepositWalletFactory.json", "src/abi/deposit_wallet_factory.rs")?;
    Ok(())
}
