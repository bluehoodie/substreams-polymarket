use anyhow::Result;
use std::fs;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    for (name, abi_path, out_path) in [
        (
            "OptimisticOracleV2",
            "abi/OptimisticOracleV2.json",
            "src/abi/optimistic_oracle_v2.rs",
        ),
        (
            "OptimisticOracleV3",
            "abi/OptimisticOracleV3.json",
            "src/abi/optimistic_oracle_v3.rs",
        ),
    ] {
        Abigen::new(name, abi_path)?
            .generate()?
            .write_to_file(out_path)?;

        let generated = fs::read_to_string(out_path)?;
        fs::write(out_path, format!("#![allow(clippy::all)]\n{generated}"))?;
    }

    Ok(())
}
