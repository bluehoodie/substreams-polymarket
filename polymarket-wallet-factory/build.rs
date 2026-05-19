use anyhow::Result;
use std::fs;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("DepositWalletFactory", "abi/DepositWalletFactory.json")?
        .generate()?
        .write_to_file("src/abi/deposit_wallet_factory.rs")?;

    let generated = fs::read_to_string("src/abi/deposit_wallet_factory.rs")?;
    fs::write(
        "src/abi/deposit_wallet_factory.rs",
        format!("#![allow(clippy::all)]\n{generated}"),
    )?;

    Ok(())
}
