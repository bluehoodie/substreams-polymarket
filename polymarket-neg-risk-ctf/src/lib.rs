// The `#[substreams::handlers::map]` macro generates an `extern "C"` shim that rebuilds
// `params: String` handler inputs from a raw `*mut u8` via `String::from_raw_parts`. The
// macro drops the annotated fn's attributes, so a fn-scoped allow cannot reach the
// generated shim — `not_unsafe_ptr_arg_deref` must be allowed at crate scope. The unsafe
// deref lives entirely in macro-generated code; our handlers are safe.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod abi;
#[allow(dead_code, clippy::all)]
pub mod pb;

use substreams::errors::Error;
use substreams_ethereum::pb::eth::v2 as eth;

use pb::polymarket::neg_risk_ctf::v1 as proto;
use pb::sf::substreams::index::v1::Keys;
use polymarket_substreams_common::{
    bigint_to_string, bigint_to_u32, build_tx_context, format_address,
};

const NEG_RISK_CTF_CONTRACT_ADDRESS: [u8; 20] =
    hex_literal::hex!("e2222d279d744050d28e00520010520000310F59");

/// Block index module. Per block it emits one `trader:<address>` key per maker/taker
/// seen in a Neg Risk CTF Exchange trade event (OrderFilled / OrdersMatched), so
/// `map_user_trades` can skip blocks a specific wallet never traded in — far more
/// selective than a contract-level key (a given wallet trades in a tiny fraction of
/// blocks).
///
/// This index is decoded from event *data* (maker/taker fields), which the
/// foundational `evt_addr:`/`evt_sig:` index cannot produce, so it stays local.
/// Contract-level (`evt_addr:`) skipping is delegated to `eth_common:index_events`.
#[substreams::handlers::map]
pub fn index_traders(blk: eth::Block) -> Result<Keys, Error> {
    use std::collections::HashSet;

    let mut set: HashSet<String> = HashSet::new();
    for log in blk.logs() {
        for key in trader_keys_for_log(log.log) {
            set.insert(key);
        }
    }
    Ok(Keys {
        keys: set.into_iter().collect(),
    })
}

/// Returns the `trader:<address>` keys contributed by a single log: the maker/taker
/// of a Neg Risk CTF Exchange trade (OrderFilled / OrdersMatched). Returns empty for
/// any log not from this package's contract — so an OrderFilled with the same topic0
/// emitted by an unrelated contract cannot produce trader keys. Pulled out of
/// `index_traders` so the per-log decode logic is unit-testable without building a
/// full `eth::Block`.
fn trader_keys_for_log(log: &eth::Log) -> Vec<String> {
    use abi::neg_risk_ctf::events::*;

    if !is_neg_risk_ctf_contract(log) {
        return Vec::new();
    }

    if OrderFilled::match_log(log) {
        if let Ok(e) = OrderFilled::decode(log) {
            return vec![trader_key(&e.maker), trader_key(&e.taker)];
        }
    } else if OrdersMatched::match_log(log) {
        if let Ok(e) = OrdersMatched::decode(log) {
            return vec![trader_key(&e.taker_order_maker)];
        }
    }
    Vec::new()
}

/// Block-index key for a trader address. Must match the `trader:` namespace used in
/// `map_user_trades` blockFilter queries (lowercase hex, `0x` prefix).
fn trader_key(addr: &[u8]) -> String {
    format!("trader:{}", format_address(addr))
}

/// Extracts the 20-byte trader addresses named in an SQE params string such as
/// `"trader:0x… || trader:0x…"`, for in-handler filtering. Tokens in other
/// namespaces (e.g. `evt_addr:`) or with a leading `-` (NOT) are ignored.
fn extract_trader_addresses(params: &str) -> Vec<Vec<u8>> {
    params
        .split([' ', '|', '&', '(', ')', '\t', '\n'])
        .filter_map(|tok| tok.trim().strip_prefix("trader:"))
        .filter_map(|h| hex::decode(h.trim_start_matches("0x")).ok())
        .filter(|b| b.len() == 20)
        .collect()
}

/// User-activity map: emits only the trades (OrderFilled / OrdersMatched) involving
/// the trader address(es) named in `params` — an SQE expression like
/// `"trader:0x… || trader:0x…"`, the same value used by the blockFilter query. The
/// blockFilter skips blocks none of the users traded in; this handler then keeps
/// only those users' trades within the surviving blocks.
#[substreams::handlers::map]
pub fn map_user_trades(params: String, blk: eth::Block) -> Result<proto::TradingEvents, Error> {
    use abi::neg_risk_ctf::events::*;

    let mut events = proto::TradingEvents::default();
    let watched = extract_trader_addresses(&params);
    if watched.is_empty() {
        return Ok(events);
    }
    let is_watched = |addr: &[u8]| watched.iter().any(|w| w.as_slice() == addr);

    for log in blk.logs() {
        if !is_neg_risk_ctf_contract(log.log) {
            continue;
        }

        if OrderFilled::match_log(log.log) {
            if let Ok(event) = OrderFilled::decode(log.log) {
                if is_watched(&event.maker) || is_watched(&event.taker) {
                    events.order_filled.push(proto::OrderFilled {
                        order_hash: event.order_hash.to_vec(),
                        maker: format_address(&event.maker),
                        taker: format_address(&event.taker),
                        side: bigint_to_u32(&event.side),
                        token_id: bigint_to_string(&event.token_id),
                        maker_amount_filled: bigint_to_string(&event.maker_amount_filled),
                        taker_amount_filled: bigint_to_string(&event.taker_amount_filled),
                        fee: bigint_to_string(&event.fee),
                        builder: event.builder.to_vec(),
                        metadata: event.metadata.to_vec(),
                        tx: Some(build_transaction_context(&blk, &log)),
                    });
                }
            }
        } else if OrdersMatched::match_log(log.log) {
            if let Ok(event) = OrdersMatched::decode(log.log) {
                if is_watched(&event.taker_order_maker) {
                    events.orders_matched.push(proto::OrdersMatched {
                        taker_order_hash: event.taker_order_hash.to_vec(),
                        taker_order_maker: format_address(&event.taker_order_maker),
                        side: bigint_to_u32(&event.side),
                        token_id: bigint_to_string(&event.token_id),
                        maker_amount_filled: bigint_to_string(&event.maker_amount_filled),
                        taker_amount_filled: bigint_to_string(&event.taker_amount_filled),
                        tx: Some(build_transaction_context(&blk, &log)),
                    });
                }
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn map_trading_events(blk: eth::Block) -> Result<proto::TradingEvents, Error> {
    use abi::neg_risk_ctf::events::*;

    let mut events = proto::TradingEvents::default();

    for log in blk.logs() {
        if !is_neg_risk_ctf_contract(log.log) {
            continue;
        }

        if OrderFilled::match_log(log.log) {
            if let Ok(event) = OrderFilled::decode(log.log) {
                events.order_filled.push(proto::OrderFilled {
                    order_hash: event.order_hash.to_vec(),
                    maker: format_address(&event.maker),
                    taker: format_address(&event.taker),
                    side: bigint_to_u32(&event.side),
                    token_id: bigint_to_string(&event.token_id),
                    maker_amount_filled: bigint_to_string(&event.maker_amount_filled),
                    taker_amount_filled: bigint_to_string(&event.taker_amount_filled),
                    fee: bigint_to_string(&event.fee),
                    builder: event.builder.to_vec(),
                    metadata: event.metadata.to_vec(),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if OrdersMatched::match_log(log.log) {
            if let Ok(event) = OrdersMatched::decode(log.log) {
                events.orders_matched.push(proto::OrdersMatched {
                    taker_order_hash: event.taker_order_hash.to_vec(),
                    taker_order_maker: format_address(&event.taker_order_maker),
                    side: bigint_to_u32(&event.side),
                    token_id: bigint_to_string(&event.token_id),
                    maker_amount_filled: bigint_to_string(&event.maker_amount_filled),
                    taker_amount_filled: bigint_to_string(&event.taker_amount_filled),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn map_fee_events(blk: eth::Block) -> Result<proto::FeeEvents, Error> {
    use abi::neg_risk_ctf::events::*;

    let mut events = proto::FeeEvents::default();

    for log in blk.logs() {
        if !is_neg_risk_ctf_contract(log.log) {
            continue;
        }

        if FeeCharged::match_log(log.log) {
            if let Ok(event) = FeeCharged::decode(log.log) {
                events.fee_charged.push(proto::FeeCharged {
                    recipient: format_address(&event.recipient),
                    amount: bigint_to_string(&event.amount),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if FeeReceiverUpdated::match_log(log.log) {
            if let Ok(event) = FeeReceiverUpdated::decode(log.log) {
                events.fee_receiver_updated.push(proto::FeeReceiverUpdated {
                    fee_receiver: format_address(&event.fee_receiver),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if MaxFeeRateUpdated::match_log(log.log) {
            if let Ok(event) = MaxFeeRateUpdated::decode(log.log) {
                events.max_fee_rate_updated.push(proto::MaxFeeRateUpdated {
                    max_fee_rate: bigint_to_string(&event.max_fee_rate),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn map_admin_events(blk: eth::Block) -> Result<proto::AdminEvents, Error> {
    use abi::neg_risk_ctf::events::*;

    let mut events = proto::AdminEvents::default();

    for log in blk.logs() {
        if !is_neg_risk_ctf_contract(log.log) {
            continue;
        }

        if NewAdmin::match_log(log.log) {
            if let Ok(event) = NewAdmin::decode(log.log) {
                events.new_admin.push(proto::NewAdmin {
                    new_admin_address: format_address(&event.new_admin_address),
                    admin: format_address(&event.admin),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if NewOperator::match_log(log.log) {
            if let Ok(event) = NewOperator::decode(log.log) {
                events.new_operator.push(proto::NewOperator {
                    new_operator_address: format_address(&event.new_operator_address),
                    admin: format_address(&event.admin),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if RemovedAdmin::match_log(log.log) {
            if let Ok(event) = RemovedAdmin::decode(log.log) {
                events.removed_admin.push(proto::RemovedAdmin {
                    removed_admin: format_address(&event.removed_admin),
                    admin: format_address(&event.admin),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if RemovedOperator::match_log(log.log) {
            if let Ok(event) = RemovedOperator::decode(log.log) {
                events.removed_operator.push(proto::RemovedOperator {
                    removed_operator: format_address(&event.removed_operator),
                    admin: format_address(&event.admin),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn map_pause_events(blk: eth::Block) -> Result<proto::PauseEvents, Error> {
    use abi::neg_risk_ctf::events::*;

    let mut events = proto::PauseEvents::default();

    for log in blk.logs() {
        if !is_neg_risk_ctf_contract(log.log) {
            continue;
        }

        if UserPaused::match_log(log.log) {
            if let Ok(event) = UserPaused::decode(log.log) {
                events.user_paused.push(proto::UserPaused {
                    user: format_address(&event.user),
                    effective_pause_block: bigint_to_string(&event.effective_pause_block),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if UserUnpaused::match_log(log.log) {
            if let Ok(event) = UserUnpaused::decode(log.log) {
                events.user_unpaused.push(proto::UserUnpaused {
                    user: format_address(&event.user),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if UserPauseBlockIntervalUpdated::match_log(log.log) {
            if let Ok(event) = UserPauseBlockIntervalUpdated::decode(log.log) {
                events.user_pause_block_interval_updated.push(
                    proto::UserPauseBlockIntervalUpdated {
                        old_interval: bigint_to_string(&event.old_interval),
                        new_interval: bigint_to_string(&event.new_interval),
                        tx: Some(build_transaction_context(&blk, &log)),
                    },
                );
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn map_approval_events(blk: eth::Block) -> Result<proto::OrderApprovalEvents, Error> {
    use abi::neg_risk_ctf::events::*;

    let mut events = proto::OrderApprovalEvents::default();

    for log in blk.logs() {
        if !is_neg_risk_ctf_contract(log.log) {
            continue;
        }

        if OrderPreapproved::match_log(log.log) {
            if let Ok(event) = OrderPreapproved::decode(log.log) {
                events.order_preapproved.push(proto::OrderPreapproved {
                    order_hash: event.order_hash.to_vec(),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if OrderPreapprovalInvalidated::match_log(log.log) {
            if let Ok(event) = OrderPreapprovalInvalidated::decode(log.log) {
                events
                    .order_preapproval_invalidated
                    .push(proto::OrderPreapprovalInvalidated {
                        order_hash: event.order_hash.to_vec(),
                        tx: Some(build_transaction_context(&blk, &log)),
                    });
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn map_all_events(blk: eth::Block) -> Result<proto::AllEvents, Error> {
    use abi::neg_risk_ctf::events::*;

    let mut trading_events = proto::TradingEvents::default();
    let mut fee_events = proto::FeeEvents::default();
    let mut admin_events = proto::AdminEvents::default();
    let mut pause_events = proto::PauseEvents::default();
    let mut approval_events = proto::OrderApprovalEvents::default();

    for log in blk.logs() {
        if !is_neg_risk_ctf_contract(log.log) {
            continue;
        }

        if OrderFilled::match_log(log.log) {
            if let Ok(event) = OrderFilled::decode(log.log) {
                trading_events.order_filled.push(proto::OrderFilled {
                    order_hash: event.order_hash.to_vec(),
                    maker: format_address(&event.maker),
                    taker: format_address(&event.taker),
                    side: bigint_to_u32(&event.side),
                    token_id: bigint_to_string(&event.token_id),
                    maker_amount_filled: bigint_to_string(&event.maker_amount_filled),
                    taker_amount_filled: bigint_to_string(&event.taker_amount_filled),
                    fee: bigint_to_string(&event.fee),
                    builder: event.builder.to_vec(),
                    metadata: event.metadata.to_vec(),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if OrdersMatched::match_log(log.log) {
            if let Ok(event) = OrdersMatched::decode(log.log) {
                trading_events.orders_matched.push(proto::OrdersMatched {
                    taker_order_hash: event.taker_order_hash.to_vec(),
                    taker_order_maker: format_address(&event.taker_order_maker),
                    side: bigint_to_u32(&event.side),
                    token_id: bigint_to_string(&event.token_id),
                    maker_amount_filled: bigint_to_string(&event.maker_amount_filled),
                    taker_amount_filled: bigint_to_string(&event.taker_amount_filled),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if FeeCharged::match_log(log.log) {
            if let Ok(event) = FeeCharged::decode(log.log) {
                fee_events.fee_charged.push(proto::FeeCharged {
                    recipient: format_address(&event.recipient),
                    amount: bigint_to_string(&event.amount),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if FeeReceiverUpdated::match_log(log.log) {
            if let Ok(event) = FeeReceiverUpdated::decode(log.log) {
                fee_events
                    .fee_receiver_updated
                    .push(proto::FeeReceiverUpdated {
                        fee_receiver: format_address(&event.fee_receiver),
                        tx: Some(build_transaction_context(&blk, &log)),
                    });
            }
        } else if MaxFeeRateUpdated::match_log(log.log) {
            if let Ok(event) = MaxFeeRateUpdated::decode(log.log) {
                fee_events
                    .max_fee_rate_updated
                    .push(proto::MaxFeeRateUpdated {
                        max_fee_rate: bigint_to_string(&event.max_fee_rate),
                        tx: Some(build_transaction_context(&blk, &log)),
                    });
            }
        } else if NewAdmin::match_log(log.log) {
            if let Ok(event) = NewAdmin::decode(log.log) {
                admin_events.new_admin.push(proto::NewAdmin {
                    new_admin_address: format_address(&event.new_admin_address),
                    admin: format_address(&event.admin),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if NewOperator::match_log(log.log) {
            if let Ok(event) = NewOperator::decode(log.log) {
                admin_events.new_operator.push(proto::NewOperator {
                    new_operator_address: format_address(&event.new_operator_address),
                    admin: format_address(&event.admin),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if RemovedAdmin::match_log(log.log) {
            if let Ok(event) = RemovedAdmin::decode(log.log) {
                admin_events.removed_admin.push(proto::RemovedAdmin {
                    removed_admin: format_address(&event.removed_admin),
                    admin: format_address(&event.admin),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if RemovedOperator::match_log(log.log) {
            if let Ok(event) = RemovedOperator::decode(log.log) {
                admin_events.removed_operator.push(proto::RemovedOperator {
                    removed_operator: format_address(&event.removed_operator),
                    admin: format_address(&event.admin),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if UserPaused::match_log(log.log) {
            if let Ok(event) = UserPaused::decode(log.log) {
                pause_events.user_paused.push(proto::UserPaused {
                    user: format_address(&event.user),
                    effective_pause_block: bigint_to_string(&event.effective_pause_block),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if UserUnpaused::match_log(log.log) {
            if let Ok(event) = UserUnpaused::decode(log.log) {
                pause_events.user_unpaused.push(proto::UserUnpaused {
                    user: format_address(&event.user),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if UserPauseBlockIntervalUpdated::match_log(log.log) {
            if let Ok(event) = UserPauseBlockIntervalUpdated::decode(log.log) {
                pause_events.user_pause_block_interval_updated.push(
                    proto::UserPauseBlockIntervalUpdated {
                        old_interval: bigint_to_string(&event.old_interval),
                        new_interval: bigint_to_string(&event.new_interval),
                        tx: Some(build_transaction_context(&blk, &log)),
                    },
                );
            }
        } else if OrderPreapproved::match_log(log.log) {
            if let Ok(event) = OrderPreapproved::decode(log.log) {
                approval_events
                    .order_preapproved
                    .push(proto::OrderPreapproved {
                        order_hash: event.order_hash.to_vec(),
                        tx: Some(build_transaction_context(&blk, &log)),
                    });
            }
        } else if OrderPreapprovalInvalidated::match_log(log.log) {
            if let Ok(event) = OrderPreapprovalInvalidated::decode(log.log) {
                approval_events.order_preapproval_invalidated.push(
                    proto::OrderPreapprovalInvalidated {
                        order_hash: event.order_hash.to_vec(),
                        tx: Some(build_transaction_context(&blk, &log)),
                    },
                );
            }
        }
    }

    Ok(proto::AllEvents {
        trading_events: if !trading_events.order_filled.is_empty()
            || !trading_events.orders_matched.is_empty()
        {
            Some(trading_events)
        } else {
            None
        },
        fee_events: if !fee_events.fee_charged.is_empty()
            || !fee_events.fee_receiver_updated.is_empty()
            || !fee_events.max_fee_rate_updated.is_empty()
        {
            Some(fee_events)
        } else {
            None
        },
        admin_events: if !admin_events.new_admin.is_empty()
            || !admin_events.new_operator.is_empty()
            || !admin_events.removed_admin.is_empty()
            || !admin_events.removed_operator.is_empty()
        {
            Some(admin_events)
        } else {
            None
        },
        pause_events: if !pause_events.user_paused.is_empty()
            || !pause_events.user_unpaused.is_empty()
            || !pause_events.user_pause_block_interval_updated.is_empty()
        {
            Some(pause_events)
        } else {
            None
        },
        order_approval_events: if !approval_events.order_preapproved.is_empty()
            || !approval_events.order_preapproval_invalidated.is_empty()
        {
            Some(approval_events)
        } else {
            None
        },
    })
}

#[inline]
fn is_neg_risk_ctf_contract(log: &eth::Log) -> bool {
    log.address == NEG_RISK_CTF_CONTRACT_ADDRESS
}

#[inline]
fn build_transaction_context(
    blk: &eth::Block,
    log: &substreams_ethereum::block_view::LogView,
) -> proto::TransactionContext {
    let ctx = build_tx_context(blk, log);
    proto::TransactionContext {
        tx_hash: ctx.tx_hash,
        log_index: ctx.log_index,
        block_number: ctx.block_number,
        timestamp: ctx.timestamp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use substreams_ethereum::pb::eth::v2 as eth;

    #[test]
    fn test_is_neg_risk_ctf_contract_matching_address() {
        let log = eth::Log {
            address: hex_literal::hex!("e2222d279d744050d28e00520010520000310F59").to_vec(),
            ..Default::default()
        };
        assert!(is_neg_risk_ctf_contract(&log));
    }

    #[test]
    fn test_is_neg_risk_ctf_contract_non_matching_address() {
        let log = eth::Log {
            address: hex_literal::hex!("0000000000000000000000000000000000000000").to_vec(),
            ..Default::default()
        };
        assert!(!is_neg_risk_ctf_contract(&log));
    }

    #[test]
    fn test_neg_risk_ctf_contract_address_is_20_bytes() {
        assert_eq!(NEG_RISK_CTF_CONTRACT_ADDRESS.len(), 20);
    }

    #[test]
    fn test_format_address() {
        let bytes = hex_literal::hex!("e2222d279d744050d28e00520010520000310F59");
        let result = format_address(&bytes);
        assert_eq!(result, "0xe2222d279d744050d28e00520010520000310f59");
    }

    #[test]
    fn test_order_filled_decodes_valid_log() {
        use crate::abi::neg_risk_ctf::events::OrderFilled;

        // keccak256("OrderFilled(bytes32,address,address,uint8,uint256,uint256,uint256,uint256,bytes32,bytes32)")
        // mirrors the generated binding's TOPIC_ID
        let topic0: Vec<u8> =
            hex_literal::hex!("d543adfd945773f1a62f74f0ee55a5e3b9b1a28262980ba90b1a89f2ea84d8ee")
                .to_vec();

        let order_hash = [0x11u8; 32];
        let maker_addr: [u8; 20] = hex_literal::hex!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
        let taker_addr: [u8; 20] = hex_literal::hex!("BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB");

        // addresses left-padded to 32 bytes in topics
        let mut maker_topic = [0u8; 32];
        maker_topic[12..].copy_from_slice(&maker_addr);
        let mut taker_topic = [0u8; 32];
        taker_topic[12..].copy_from_slice(&taker_addr);

        // 7 non-indexed fields ABI-encoded as 32-byte words (big-endian right-aligned)
        let u256_word = |v: u64| -> [u8; 32] {
            let mut w = [0u8; 32];
            w[24..].copy_from_slice(&v.to_be_bytes());
            w
        };
        let side: u64 = 1;
        let token_id: u64 = 0x1234;
        let maker_amount_filled: u64 = 1_000_000;
        let taker_amount_filled: u64 = 2_000_000;
        let fee: u64 = 500;
        let builder = [0xAAu8; 32];
        let metadata = [0xBBu8; 32];

        let mut data = Vec::with_capacity(224);
        data.extend_from_slice(&u256_word(side));
        data.extend_from_slice(&u256_word(token_id));
        data.extend_from_slice(&u256_word(maker_amount_filled));
        data.extend_from_slice(&u256_word(taker_amount_filled));
        data.extend_from_slice(&u256_word(fee));
        data.extend_from_slice(&builder);
        data.extend_from_slice(&metadata);
        assert_eq!(data.len(), 224);

        let log = eth::Log {
            address: NEG_RISK_CTF_CONTRACT_ADDRESS.to_vec(),
            topics: vec![
                topic0,
                order_hash.to_vec(),
                maker_topic.to_vec(),
                taker_topic.to_vec(),
            ],
            data,
            ..Default::default()
        };

        assert!(
            OrderFilled::match_log(&log),
            "match_log must return true for valid log"
        );

        let decoded = OrderFilled::decode(&log).expect("decode must succeed for valid log");
        assert_eq!(decoded.order_hash, order_hash);
        assert_eq!(decoded.maker, maker_addr.to_vec());
        assert_eq!(decoded.taker, taker_addr.to_vec());
        assert_eq!(decoded.side, substreams::scalar::BigInt::from(side));
        assert_eq!(decoded.token_id, substreams::scalar::BigInt::from(token_id));
        assert_eq!(
            decoded.maker_amount_filled,
            substreams::scalar::BigInt::from(maker_amount_filled)
        );
        assert_eq!(
            decoded.taker_amount_filled,
            substreams::scalar::BigInt::from(taker_amount_filled)
        );
        assert_eq!(decoded.fee, substreams::scalar::BigInt::from(fee));
        assert_eq!(decoded.builder, builder);
        assert_eq!(decoded.metadata, metadata);
    }

    #[test]
    fn test_order_filled_rejects_wrong_topic() {
        use crate::abi::neg_risk_ctf::events::OrderFilled;

        // keccak256("OrderFilled(bytes32,address,address,uint8,uint256,uint256,uint256,uint256,bytes32,bytes32)")
        // mirrors the generated binding's TOPIC_ID
        let mut topic0: Vec<u8> =
            hex_literal::hex!("d543adfd945773f1a62f74f0ee55a5e3b9b1a28262980ba90b1a89f2ea84d8ee")
                .to_vec();

        let order_hash = [0x11u8; 32];
        let maker_addr: [u8; 20] = hex_literal::hex!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
        let taker_addr: [u8; 20] = hex_literal::hex!("BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB");

        let mut maker_topic = [0u8; 32];
        maker_topic[12..].copy_from_slice(&maker_addr);
        let mut taker_topic = [0u8; 32];
        taker_topic[12..].copy_from_slice(&taker_addr);

        let u256_word = |v: u64| -> [u8; 32] {
            let mut w = [0u8; 32];
            w[24..].copy_from_slice(&v.to_be_bytes());
            w
        };
        let mut data = Vec::with_capacity(224);
        data.extend_from_slice(&u256_word(1));
        data.extend_from_slice(&u256_word(0x1234));
        data.extend_from_slice(&u256_word(1_000_000));
        data.extend_from_slice(&u256_word(2_000_000));
        data.extend_from_slice(&u256_word(500));
        data.extend_from_slice(&[0xAAu8; 32]);
        data.extend_from_slice(&[0xBBu8; 32]);

        // flip first byte of topic0 to simulate wrong event signature
        topic0[0] ^= 0xFF;

        let log = eth::Log {
            address: NEG_RISK_CTF_CONTRACT_ADDRESS.to_vec(),
            topics: vec![
                topic0,
                order_hash.to_vec(),
                maker_topic.to_vec(),
                taker_topic.to_vec(),
            ],
            data,
            ..Default::default()
        };

        assert!(
            !OrderFilled::match_log(&log),
            "match_log must return false for wrong topic0"
        );
    }

    #[test]
    fn test_fee_charged_decodes_valid_log() {
        use crate::abi::neg_risk_ctf::events::FeeCharged;

        // keccak256("FeeCharged(address,uint256)")
        // mirrors the generated binding's TOPIC_ID
        let topic0: Vec<u8> =
            hex_literal::hex!("55bb3cade9d43b798a4fe5ffdd05024b2d7870df53920673bfc7e68047cd0ab1")
                .to_vec();

        let recipient_addr: [u8; 20] =
            hex_literal::hex!("CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC");
        // indexed address is left-padded to 32 bytes in topics
        let mut recipient_topic = [0u8; 32];
        recipient_topic[12..].copy_from_slice(&recipient_addr);

        // single non-indexed uint256 field (amount), ABI-encoded as one 32-byte word
        let amount: u64 = 81_580;
        let mut data = [0u8; 32];
        data[24..].copy_from_slice(&amount.to_be_bytes());

        let log = eth::Log {
            address: NEG_RISK_CTF_CONTRACT_ADDRESS.to_vec(),
            topics: vec![topic0, recipient_topic.to_vec()],
            data: data.to_vec(),
            ..Default::default()
        };

        assert!(
            FeeCharged::match_log(&log),
            "match_log must return true for valid FeeCharged log"
        );

        let decoded = FeeCharged::decode(&log).expect("decode must succeed for valid log");
        assert_eq!(decoded.recipient, recipient_addr.to_vec());
        assert_eq!(decoded.amount, substreams::scalar::BigInt::from(amount));
    }
}

#[cfg(test)]
mod index_tests {
    use super::*;

    // keccak256("OrderFilled(bytes32,address,address,uint8,uint256,uint256,uint256,uint256,bytes32,bytes32)")
    const ORDER_FILLED_TOPIC0: [u8; 32] =
        hex_literal::hex!("d543adfd945773f1a62f74f0ee55a5e3b9b1a28262980ba90b1a89f2ea84d8ee");

    fn order_filled_log(contract: [u8; 20], maker: [u8; 20], taker: [u8; 20]) -> eth::Log {
        let mut maker_topic = [0u8; 32];
        maker_topic[12..].copy_from_slice(&maker);
        let mut taker_topic = [0u8; 32];
        taker_topic[12..].copy_from_slice(&taker);

        // 7 non-indexed uint256/bytes32 words of arbitrary value — decode only needs
        // the right shape, not specific values, for the trader-key extraction.
        let data = vec![0u8; 224];

        eth::Log {
            address: contract.to_vec(),
            topics: vec![
                ORDER_FILLED_TOPIC0.to_vec(),
                [0x11u8; 32].to_vec(),
                maker_topic.to_vec(),
                taker_topic.to_vec(),
            ],
            data,
            ..Default::default()
        }
    }

    #[test]
    fn test_trader_keys_for_our_contract_order_filled() {
        let maker = hex_literal::hex!("00000000000000000000000000000000000000ab");
        let taker = hex_literal::hex!("00000000000000000000000000000000000000cd");
        let log = order_filled_log(NEG_RISK_CTF_CONTRACT_ADDRESS, maker, taker);

        assert_eq!(
            trader_keys_for_log(&log),
            vec![
                "trader:0x00000000000000000000000000000000000000ab".to_string(),
                "trader:0x00000000000000000000000000000000000000cd".to_string(),
            ]
        );
    }

    #[test]
    fn test_trader_keys_skips_unrelated_contract() {
        let unrelated = hex_literal::hex!("00000000000000000000000000000000000000ff");
        let maker = hex_literal::hex!("00000000000000000000000000000000000000ab");
        let taker = hex_literal::hex!("00000000000000000000000000000000000000cd");
        // Same OrderFilled topic0, but emitted by a different contract: must yield no keys.
        let log = order_filled_log(unrelated, maker, taker);

        assert!(trader_keys_for_log(&log).is_empty());
    }

    #[test]
    fn test_trader_keys_skips_non_trade_log() {
        let log = eth::Log {
            address: NEG_RISK_CTF_CONTRACT_ADDRESS.to_vec(),
            topics: vec![[0x00u8; 32].to_vec()],
            ..Default::default()
        };
        assert!(trader_keys_for_log(&log).is_empty());
    }
}

#[cfg(test)]
mod user_index_tests {
    use super::*;

    #[test]
    fn test_trader_key() {
        let addr = hex_literal::hex!("00000000000000000000000000000000000000ab");
        assert_eq!(
            trader_key(&addr),
            "trader:0x00000000000000000000000000000000000000ab"
        );
    }

    #[test]
    fn test_extract_trader_addresses_single() {
        let got = extract_trader_addresses("trader:0x00000000000000000000000000000000000000ab");
        assert_eq!(
            got,
            vec![hex_literal::hex!("00000000000000000000000000000000000000ab").to_vec()]
        );
    }

    #[test]
    fn test_extract_trader_addresses_or_list() {
        let got = extract_trader_addresses(
            "trader:0x00000000000000000000000000000000000000ab || trader:0x00000000000000000000000000000000000000cd",
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
    fn test_extract_trader_addresses_ignores_other_namespaces() {
        let got = extract_trader_addresses(
            "evt_addr:0xdeadbeef || trader:0x00000000000000000000000000000000000000ab",
        );
        assert_eq!(
            got,
            vec![hex_literal::hex!("00000000000000000000000000000000000000ab").to_vec()]
        );
    }

    #[test]
    fn test_extract_trader_addresses_empty() {
        assert!(extract_trader_addresses("").is_empty());
    }
}
