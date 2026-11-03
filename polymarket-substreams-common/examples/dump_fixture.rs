//! Writes the synthetic bench block (`benches/fixture.rs`) as protobuf bytes, the
//! input `scripts/wasm_bench.js` feeds to a built handler.
//!
//! `cargo run -p polymarket-substreams-common --example dump_fixture -- block.bin`
#[path = "../benches/fixture.rs"]
mod fixture;

use buffa::Message;

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: dump_fixture <out.bin>");
        std::process::exit(2);
    };
    if let Err(e) = std::fs::write(&path, fixture::block().encode_to_vec()) {
        eprintln!("writing {path}: {e}");
        std::process::exit(1);
    }
}
