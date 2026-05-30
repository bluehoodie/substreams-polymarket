#[allow(dead_code, clippy::all)]
pub mod pb;
pub mod abi;

use std::collections::HashSet;

use substreams::errors::Error;
use substreams_ethereum::pb::eth::v2 as eth;

use pb::sf::substreams::index::v1::Keys;
use polymarket_substreams_common::format_address;

// All Polymarket contracts that carry user-attributable activity. Resolution
// (UMA oracle) contracts are intentionally excluded — disputes/proposals are
// protocol-level, not a wallet's trading footprint.
const CTF_EXCHANGE: [u8; 20] = hex_literal::hex!("E111180000d2663C0091e4f400237545B87B996B");
const NEG_RISK_CTF: [u8; 20] = hex_literal::hex!("e2222d279d744050d28e00520010520000310F59");
const CONDITIONAL_TOKENS: [u8; 20] = hex_literal::hex!("4D97DCd97eC945f40cF65F87097ACe5EA0476045");
const NEG_RISK_ADAPTER: [u8; 20] = hex_literal::hex!("d91E80cF2E7be2e162c6513ceD06f1dD0dA35296");
const PUSD: [u8; 20] = hex_literal::hex!("C011a7E12a19f7B1f670d46F03B03f3342E82DFB");
const CTF_COLLATERAL_ADAPTER: [u8; 20] = hex_literal::hex!("AdA100Db00Ca00073811820692005400218FcE1f");
const NEG_RISK_CTF_COLLATERAL_ADAPTER: [u8; 20] = hex_literal::hex!("adA2005600Dec949baf300f4C6120000bDB6eAab");
const DEPOSIT_WALLET_FACTORY: [u8; 20] = hex_literal::hex!("00000000000Fb5C9ADea0298D729A0CB3823Cc07");

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
        .split(|c| matches!(c, ' ' | '|' | '&' | '(' | ')' | '\t' | '\n'))
        .filter_map(|tok| tok.trim().strip_prefix("user:"))
        .filter_map(|h| hex::decode(h.trim_start_matches("0x")).ok())
        .filter(|b| b.len() == 20)
        .collect()
}

/// Consumer of the `index_users` block index. Its `blockFilter` skips every block
/// none of the watched wallets (from `params`, e.g. `"user:0x… || user:0x…"`)
/// touched; for the surviving blocks it emits the watched wallets that were active.
/// Pair with the per-contract packages (e.g. `polymarket-exchange:map_user_trades`)
/// to fetch the full event details for those blocks.
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
    let keys: Vec<String> = collect_user_keys(&blk)
        .into_iter()
        .filter(|k| watched.contains(k))
        .collect();
    Ok(Keys { keys })
}

/// Foundational block index of Polymarket user activity. For each block it emits a
/// `user:<address>` key for every wallet involved in any action across all
/// Polymarket contracts — trades, position splits/merges, redemptions,
/// conversions, pUSD transfers, ERC1155 transfers, and wallet deployments.
#[substreams::handlers::map]
pub fn index_users(blk: eth::Block) -> Result<Keys, Error> {
    Ok(Keys { keys: collect_user_keys(&blk) })
}

fn collect_user_keys(blk: &eth::Block) -> Vec<String> {
    let mut set: HashSet<String> = HashSet::new();
    {
        let mut add = |a: &[u8]| {
            set.insert(user_key(a));
        };

        for log in blk.logs() {
            let addr: &[u8] = &log.log.address;

            if addr == CTF_EXCHANGE {
                use abi::ctf_exchange::events as ex;
                if ex::OrderFilled::match_log(log.log) {
                    if let Ok(e) = ex::OrderFilled::decode(log.log) {
                        add(&e.maker);
                        add(&e.taker);
                    }
                } else if ex::OrdersMatched::match_log(log.log) {
                    if let Ok(e) = ex::OrdersMatched::decode(log.log) {
                        add(&e.taker_order_maker);
                    }
                }
            } else if addr == NEG_RISK_CTF {
                use abi::neg_risk_ctf::events as nr;
                if nr::OrderFilled::match_log(log.log) {
                    if let Ok(e) = nr::OrderFilled::decode(log.log) {
                        add(&e.maker);
                        add(&e.taker);
                    }
                } else if nr::OrdersMatched::match_log(log.log) {
                    if let Ok(e) = nr::OrdersMatched::decode(log.log) {
                        add(&e.taker_order_maker);
                    }
                }
            } else if addr == CONDITIONAL_TOKENS {
                use abi::conditional_tokens::events as ct;
                if ct::PositionSplit::match_log(log.log) {
                    if let Ok(e) = ct::PositionSplit::decode(log.log) {
                        add(&e.stakeholder);
                    }
                } else if ct::PositionsMerge::match_log(log.log) {
                    if let Ok(e) = ct::PositionsMerge::decode(log.log) {
                        add(&e.stakeholder);
                    }
                } else if ct::PayoutRedemption::match_log(log.log) {
                    if let Ok(e) = ct::PayoutRedemption::decode(log.log) {
                        add(&e.redeemer);
                    }
                } else if ct::TransferSingle::match_log(log.log) {
                    if let Ok(e) = ct::TransferSingle::decode(log.log) {
                        add(&e.from);
                        add(&e.to);
                        add(&e.operator);
                    }
                } else if ct::TransferBatch::match_log(log.log) {
                    if let Ok(e) = ct::TransferBatch::decode(log.log) {
                        add(&e.from);
                        add(&e.to);
                        add(&e.operator);
                    }
                } else if ct::ApprovalForAll::match_log(log.log) {
                    if let Ok(e) = ct::ApprovalForAll::decode(log.log) {
                        add(&e.owner);
                    }
                }
            } else if addr == NEG_RISK_ADAPTER {
                use abi::neg_risk_adapter::events as na;
                if na::PositionSplit::match_log(log.log) {
                    if let Ok(e) = na::PositionSplit::decode(log.log) {
                        add(&e.stakeholder);
                    }
                } else if na::PositionsMerge::match_log(log.log) {
                    if let Ok(e) = na::PositionsMerge::decode(log.log) {
                        add(&e.stakeholder);
                    }
                } else if na::PositionsConverted::match_log(log.log) {
                    if let Ok(e) = na::PositionsConverted::decode(log.log) {
                        add(&e.stakeholder);
                    }
                } else if na::PayoutRedemption::match_log(log.log) {
                    if let Ok(e) = na::PayoutRedemption::decode(log.log) {
                        add(&e.redeemer);
                    }
                }
            } else if addr == PUSD {
                use abi::p_usd::events as pu;
                if pu::Transfer::match_log(log.log) {
                    if let Ok(e) = pu::Transfer::decode(log.log) {
                        add(&e.from);
                        add(&e.to);
                    }
                } else if pu::Wrapped::match_log(log.log) {
                    if let Ok(e) = pu::Wrapped::decode(log.log) {
                        add(&e.caller);
                        add(&e.to);
                    }
                } else if pu::Unwrapped::match_log(log.log) {
                    if let Ok(e) = pu::Unwrapped::decode(log.log) {
                        add(&e.caller);
                        add(&e.to);
                    }
                }
            } else if addr == CTF_COLLATERAL_ADAPTER {
                use abi::ctf_collateral_adapter::events as ca;
                if ca::PositionSplit::match_log(log.log) {
                    if let Ok(e) = ca::PositionSplit::decode(log.log) {
                        add(&e.initiator);
                    }
                } else if ca::PositionsMerged::match_log(log.log) {
                    if let Ok(e) = ca::PositionsMerged::decode(log.log) {
                        add(&e.initiator);
                    }
                } else if ca::PositionsRedeemed::match_log(log.log) {
                    if let Ok(e) = ca::PositionsRedeemed::decode(log.log) {
                        add(&e.initiator);
                    }
                }
            } else if addr == NEG_RISK_CTF_COLLATERAL_ADAPTER {
                use abi::neg_risk_ctf_collateral_adapter::events as nca;
                if nca::PositionSplit::match_log(log.log) {
                    if let Ok(e) = nca::PositionSplit::decode(log.log) {
                        add(&e.initiator);
                    }
                } else if nca::PositionsMerged::match_log(log.log) {
                    if let Ok(e) = nca::PositionsMerged::decode(log.log) {
                        add(&e.initiator);
                    }
                } else if nca::PositionsRedeemed::match_log(log.log) {
                    if let Ok(e) = nca::PositionsRedeemed::decode(log.log) {
                        add(&e.initiator);
                    }
                } else if nca::PositionsConverted::match_log(log.log) {
                    if let Ok(e) = nca::PositionsConverted::decode(log.log) {
                        add(&e.initiator);
                    }
                }
            } else if addr == DEPOSIT_WALLET_FACTORY {
                use abi::deposit_wallet_factory::events as wf;
                if wf::WalletDeployed::match_log(log.log) {
                    if let Ok(e) = wf::WalletDeployed::decode(log.log) {
                        add(&e.wallet);
                        add(&e.owner);
                    }
                }
            }
        }
    }

    // Emit keys in a deterministic (sorted) order. HashSet iteration order is
    // unspecified; a stable order keeps the cached index reproducible across runs.
    let mut keys: Vec<String> = set.into_iter().collect();
    keys.sort();
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_key() {
        let addr = hex_literal::hex!("00000000000000000000000000000000000000ab");
        assert_eq!(user_key(&addr), "user:0x00000000000000000000000000000000000000ab");
    }

    #[test]
    fn test_user_key_lowercases() {
        // log addresses arrive as raw bytes; hex::encode is always lowercase
        let addr = hex_literal::hex!("E111180000d2663C0091e4f400237545B87B996B");
        assert_eq!(user_key(&addr), "user:0xe111180000d2663c0091e4f400237545b87b996b");
    }

    #[test]
    fn test_all_contracts_distinct_and_20_bytes() {
        let addrs: [&[u8; 20]; 8] = [
            &CTF_EXCHANGE,
            &NEG_RISK_CTF,
            &CONDITIONAL_TOKENS,
            &NEG_RISK_ADAPTER,
            &PUSD,
            &CTF_COLLATERAL_ADAPTER,
            &NEG_RISK_CTF_COLLATERAL_ADAPTER,
            &DEPOSIT_WALLET_FACTORY,
        ];
        for (i, a) in addrs.iter().enumerate() {
            assert_eq!(a.len(), 20);
            for b in addrs.iter().skip(i + 1) {
                assert_ne!(a, b);
            }
        }
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
            extract_user_addresses("trader:0xdead || user:0x00000000000000000000000000000000000000ab"),
            vec![hex_literal::hex!("00000000000000000000000000000000000000ab").to_vec()]
        );
    }
}
