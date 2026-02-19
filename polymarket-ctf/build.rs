use anyhow::Result;
use std::fs;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("ConditionalTokens", "abi/ConditionalTokens.json")?
        .generate()?
        .write_to_file("src/abi/conditional_tokens.rs")?;

    let generated = fs::read_to_string("src/abi/conditional_tokens.rs")?;
    fs::write(
        "src/abi/conditional_tokens.rs",
        format!("#![allow(clippy::all)]\n{generated}"),
    )?;

    Ok(())
}
