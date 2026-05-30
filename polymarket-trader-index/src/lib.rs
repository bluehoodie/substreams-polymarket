// The `#[substreams::handlers::map]` macro generates an `extern "C"` shim that rebuilds
// `params: String` handler inputs from a raw `*mut u8` via `String::from_raw_parts`. The
// macro drops the annotated fn's attributes, so a fn-scoped allow cannot reach the
// generated shim — `not_unsafe_ptr_arg_deref` must be allowed at crate scope. The unsafe
// deref lives entirely in macro-generated code; our handlers are safe.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

#[allow(dead_code, clippy::all)]
pub mod pb;

use substreams::errors::Error;
use substreams_ethereum::pb::eth::v2 as eth;

use pb::sf::substreams::index::v1::Keys;
use polymarket_substreams_common::{collect_user_addresses, format_address};

/// Block-index key for a wallet that was active in the block. Consumers query
/// this namespace, e.g. `user:0xabc… || user:0xdef…`, to skip blocks none of the
/// watched wallets touched. Lowercase hex with `0x` prefix.
fn user_key(addr: &[u8]) -> String {
    format!("user:{}", format_address(addr))
}

/// Parses the watched wallet addresses out of an SQE params string such as
/// `"user:0x… || user:0x…"` (the same value used by the blockFilter query).
fn extract_user_addresses(params: &str) -> Vec<Vec<u8>> {
    params
        .split([' ', '|', '&', '(', ')', '\t', '\n'])
        .filter_map(|tok| tok.trim().strip_prefix("user:"))
        .filter_map(|h| hex::decode(h.trim_start_matches("0x")).ok())
        .filter(|b| b.len() == 20)
        .collect()
}

/// Warming gate (see manifest `map_user_keys`). Its `blockFilter` reuses the
/// already-warm foundational `eth_common:index_events`, so this runs only on
/// blocks where a Polymarket contract emitted an event. For those blocks it
/// emits a `user:<address>` key for every wallet involved in any action across
/// all Polymarket contracts — trades, position splits/merges, redemptions,
/// conversions, pUSD transfers, ERC1155 transfers, and wallet deployments.
/// The per-contract decode lives in `polymarket-substreams-common`.
#[substreams::handlers::map]
pub fn map_user_keys(blk: eth::Block) -> Result<Keys, Error> {
    Ok(Keys {
        keys: collect_user_addresses(&blk)
            .iter()
            .map(|a| user_key(a))
            .collect(),
    })
}

/// Foundational block index of Polymarket user activity. Pass-through of
/// `map_user_keys`: republishes its `user:<address>` keys as the indexable
/// `Keys`. Carrying the work in `map_user_keys` (which holds the `blockFilter`)
/// lets this index inherit foundational-index skipping — a `blockIndex` module
/// cannot declare a `blockFilter` of its own.
#[substreams::handlers::map]
pub fn index_users(keys: Keys) -> Result<Keys, Error> {
    Ok(keys)
}

/// Consumer of the `index_users` block index. Its `blockFilter` skips every block
/// none of the watched wallets (from `params`, e.g. `"user:0x… || user:0x…"`)
/// touched; for the surviving blocks it emits the watched wallets that were active.
/// Pair with the per-contract event maps (e.g. `polymarket-exchange:map_all_events`,
/// `polymarket-ctf:map_all_events`) to fetch full decoded event details for those blocks.
///
/// NOTE: the index must be warm (computed once in `--production-mode`) before the
/// filter returns matches; a cold first run builds the index and returns nothing.
#[substreams::handlers::map]
pub fn map_user_activity(params: String, blk: eth::Block) -> Result<Keys, Error> {
    let watched: Vec<String> = extract_user_addresses(&params)
        .iter()
        .map(|a| user_key(a))
        .collect();
    if watched.is_empty() {
        return Ok(Keys::default());
    }
    let keys: Vec<String> = collect_user_addresses(&blk)
        .iter()
        .map(|a| user_key(a))
        .filter(|k| watched.contains(k))
        .collect();
    Ok(Keys { keys })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_key() {
        let addr = hex_literal::hex!("00000000000000000000000000000000000000ab");
        assert_eq!(
            user_key(&addr),
            "user:0x00000000000000000000000000000000000000ab"
        );
    }

    #[test]
    fn test_user_key_lowercases() {
        // log addresses arrive as raw bytes; hex::encode is always lowercase
        let addr = hex_literal::hex!("E111180000d2663C0091e4f400237545B87B996B");
        assert_eq!(
            user_key(&addr),
            "user:0xe111180000d2663c0091e4f400237545b87b996b"
        );
    }
}

#[cfg(test)]
mod user_activity_tests {
    use super::*;

    #[test]
    fn test_extract_user_addresses_or_list() {
        let got = extract_user_addresses(
            "user:0x00000000000000000000000000000000000000ab || user:0x00000000000000000000000000000000000000cd",
        );
        assert_eq!(
            got,
            vec![
                hex_literal::hex!("00000000000000000000000000000000000000ab").to_vec(),
                hex_literal::hex!("00000000000000000000000000000000000000cd").to_vec(),
            ]
        );
    }

    #[test]
    fn test_extract_user_addresses_ignores_other_namespaces_and_empty() {
        assert!(extract_user_addresses("").is_empty());
        assert_eq!(
            extract_user_addresses(
                "trader:0xdead || user:0x00000000000000000000000000000000000000ab"
            ),
            vec![hex_literal::hex!("00000000000000000000000000000000000000ab").to_vec()]
        );
    }
}
