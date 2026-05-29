#[allow(dead_code, clippy::all)]
pub mod pb;
pub mod abi;

use substreams::errors::Error;
use substreams_ethereum::pb::eth::v2 as eth;

use pb::polymarket::neg_risk_adapter::v1 as proto;
use pb::sf::substreams::index::v1::Keys;
use polymarket_substreams_common::{bigint_to_string, build_tx_context, format_address};

const NEG_RISK_ADAPTER_CONTRACT_ADDRESS: [u8; 20] = hex_literal::hex!("d91E80cF2E7be2e162c6513ceD06f1dD0dA35296");

/// Block index module: emits one `evt_addr:<address>` key per block containing a
/// Neg Risk Adapter contract log, so downstream modules can skip blocks that never
/// touch it (the data is sparse — a single contract on Polygon).
#[substreams::handlers::map]
pub fn index_events(blk: eth::Block) -> Result<Keys, Error> {
    let mut keys = Keys::default();
    for log in blk.logs() {
        if let Some(key) = index_key_for_address(&log.log.address) {
            if !keys.keys.contains(&key) {
                keys.keys.push(key);
            }
        }
    }
    Ok(keys)
}

/// Returns the block-index key for a log address, or `None` if it is not a
/// contract this package targets. The returned string must match the
/// `blockFilter` query in `substreams.yaml` exactly (lowercase hex, `0x` prefix).
fn index_key_for_address(addr: &[u8]) -> Option<String> {
    if addr == NEG_RISK_ADAPTER_CONTRACT_ADDRESS {
        Some(format!("evt_addr:{}", format_address(addr)))
    } else {
        None
    }
}

#[substreams::handlers::map]
pub fn map_market_events(blk: eth::Block) -> Result<proto::MarketEvents, Error> {
    use abi::neg_risk_adapter::events::*;

    let mut events = proto::MarketEvents::default();

    for log in blk.logs() {
        if !is_adapter_contract(log.log) {
            continue;
        }

        if MarketPrepared::match_log(log.log) {
            if let Ok(event) = MarketPrepared::decode(log.log) {
                events.market_prepared.push(proto::MarketPrepared {
                    market_id: event.market_id.to_vec(),
                    oracle: format_address(&event.oracle),
                    fee_bips: bigint_to_string(&event.fee_bips),
                    data: event.data,
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if QuestionPrepared::match_log(log.log) {
            if let Ok(event) = QuestionPrepared::decode(log.log) {
                events.question_prepared.push(proto::QuestionPrepared {
                    market_id: event.market_id.to_vec(),
                    question_id: event.question_id.to_vec(),
                    index: bigint_to_string(&event.index),
                    data: event.data,
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if OutcomeReported::match_log(log.log) {
            if let Ok(event) = OutcomeReported::decode(log.log) {
                events.outcome_reported.push(proto::OutcomeReported {
                    market_id: event.market_id.to_vec(),
                    question_id: event.question_id.to_vec(),
                    outcome: event.outcome,
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn map_trading_events(blk: eth::Block) -> Result<proto::TradingEvents, Error> {
    use abi::neg_risk_adapter::events::*;

    let mut events = proto::TradingEvents::default();

    for log in blk.logs() {
        if !is_adapter_contract(log.log) {
            continue;
        }

        if PositionSplit::match_log(log.log) {
            if let Ok(event) = PositionSplit::decode(log.log) {
                events.position_split.push(proto::PositionSplit {
                    stakeholder: format_address(&event.stakeholder),
                    condition_id: event.condition_id.to_vec(),
                    amount: bigint_to_string(&event.amount),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if PositionsMerge::match_log(log.log) {
            if let Ok(event) = PositionsMerge::decode(log.log) {
                events.positions_merge.push(proto::PositionsMerge {
                    stakeholder: format_address(&event.stakeholder),
                    condition_id: event.condition_id.to_vec(),
                    amount: bigint_to_string(&event.amount),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if PositionsConverted::match_log(log.log) {
            if let Ok(event) = PositionsConverted::decode(log.log) {
                events.positions_converted.push(proto::PositionsConverted {
                    stakeholder: format_address(&event.stakeholder),
                    market_id: event.market_id.to_vec(),
                    index_set: bigint_to_string(&event.index_set),
                    amount: bigint_to_string(&event.amount),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if PayoutRedemption::match_log(log.log) {
            if let Ok(event) = PayoutRedemption::decode(log.log) {
                events.payout_redemption.push(proto::PayoutRedemption {
                    redeemer: format_address(&event.redeemer),
                    condition_id: event.condition_id.to_vec(),
                    amounts: event.amounts.iter().map(bigint_to_string).collect(),
                    payout: bigint_to_string(&event.payout),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn map_admin_events(blk: eth::Block) -> Result<proto::AdminEvents, Error> {
    use abi::neg_risk_adapter::events::*;

    let mut events = proto::AdminEvents::default();

    for log in blk.logs() {
        if !is_adapter_contract(log.log) {
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
        } else if RemovedAdmin::match_log(log.log) {
            if let Ok(event) = RemovedAdmin::decode(log.log) {
                events.removed_admin.push(proto::RemovedAdmin {
                    removed_admin: format_address(&event.removed_admin),
                    admin: format_address(&event.admin),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn map_all_events(blk: eth::Block) -> Result<proto::AllEvents, Error> {
    use abi::neg_risk_adapter::events::*;

    let mut market_events = proto::MarketEvents::default();
    let mut trading_events = proto::TradingEvents::default();
    let mut admin_events = proto::AdminEvents::default();

    for log in blk.logs() {
        if !is_adapter_contract(log.log) {
            continue;
        }

        if MarketPrepared::match_log(log.log) {
            if let Ok(event) = MarketPrepared::decode(log.log) {
                market_events.market_prepared.push(proto::MarketPrepared {
                    market_id: event.market_id.to_vec(),
                    oracle: format_address(&event.oracle),
                    fee_bips: bigint_to_string(&event.fee_bips),
                    data: event.data,
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if QuestionPrepared::match_log(log.log) {
            if let Ok(event) = QuestionPrepared::decode(log.log) {
                market_events.question_prepared.push(proto::QuestionPrepared {
                    market_id: event.market_id.to_vec(),
                    question_id: event.question_id.to_vec(),
                    index: bigint_to_string(&event.index),
                    data: event.data,
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if OutcomeReported::match_log(log.log) {
            if let Ok(event) = OutcomeReported::decode(log.log) {
                market_events.outcome_reported.push(proto::OutcomeReported {
                    market_id: event.market_id.to_vec(),
                    question_id: event.question_id.to_vec(),
                    outcome: event.outcome,
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if PositionSplit::match_log(log.log) {
            if let Ok(event) = PositionSplit::decode(log.log) {
                trading_events.position_split.push(proto::PositionSplit {
                    stakeholder: format_address(&event.stakeholder),
                    condition_id: event.condition_id.to_vec(),
                    amount: bigint_to_string(&event.amount),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if PositionsMerge::match_log(log.log) {
            if let Ok(event) = PositionsMerge::decode(log.log) {
                trading_events.positions_merge.push(proto::PositionsMerge {
                    stakeholder: format_address(&event.stakeholder),
                    condition_id: event.condition_id.to_vec(),
                    amount: bigint_to_string(&event.amount),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if PositionsConverted::match_log(log.log) {
            if let Ok(event) = PositionsConverted::decode(log.log) {
                trading_events.positions_converted.push(proto::PositionsConverted {
                    stakeholder: format_address(&event.stakeholder),
                    market_id: event.market_id.to_vec(),
                    index_set: bigint_to_string(&event.index_set),
                    amount: bigint_to_string(&event.amount),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if PayoutRedemption::match_log(log.log) {
            if let Ok(event) = PayoutRedemption::decode(log.log) {
                trading_events.payout_redemption.push(proto::PayoutRedemption {
                    redeemer: format_address(&event.redeemer),
                    condition_id: event.condition_id.to_vec(),
                    amounts: event.amounts.iter().map(bigint_to_string).collect(),
                    payout: bigint_to_string(&event.payout),
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
        } else if RemovedAdmin::match_log(log.log) {
            if let Ok(event) = RemovedAdmin::decode(log.log) {
                admin_events.removed_admin.push(proto::RemovedAdmin {
                    removed_admin: format_address(&event.removed_admin),
                    admin: format_address(&event.admin),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
    }

    Ok(proto::AllEvents {
        market_events: if !market_events.market_prepared.is_empty()
            || !market_events.question_prepared.is_empty()
            || !market_events.outcome_reported.is_empty()
        {
            Some(market_events)
        } else {
            None
        },
        trading_events: if !trading_events.position_split.is_empty()
            || !trading_events.positions_merge.is_empty()
            || !trading_events.positions_converted.is_empty()
            || !trading_events.payout_redemption.is_empty()
        {
            Some(trading_events)
        } else {
            None
        },
        admin_events: if !admin_events.new_admin.is_empty()
            || !admin_events.removed_admin.is_empty()
        {
            Some(admin_events)
        } else {
            None
        },
    })
}

#[inline]
fn is_adapter_contract(log: &eth::Log) -> bool {
    log.address == NEG_RISK_ADAPTER_CONTRACT_ADDRESS
}

#[inline]
fn build_transaction_context(blk: &eth::Block, log: &substreams_ethereum::block_view::LogView) -> proto::TransactionContext {
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
    fn test_is_adapter_contract_matching_address() {
        let log = eth::Log {
            address: hex_literal::hex!("d91E80cF2E7be2e162c6513ceD06f1dD0dA35296").to_vec(),
            ..Default::default()
        };
        assert!(is_adapter_contract(&log));
    }

    #[test]
    fn test_is_adapter_contract_non_matching_address() {
        let log = eth::Log {
            address: hex_literal::hex!("0000000000000000000000000000000000000000").to_vec(),
            ..Default::default()
        };
        assert!(!is_adapter_contract(&log));
    }

    #[test]
    fn test_adapter_contract_address_is_20_bytes() {
        assert_eq!(NEG_RISK_ADAPTER_CONTRACT_ADDRESS.len(), 20);
    }

    #[test]
    fn test_format_address() {
        let bytes = hex_literal::hex!("d91E80cF2E7be2e162c6513ceD06f1dD0dA35296");
        let result = format_address(&bytes);
        assert_eq!(result, "0xd91e80cf2e7be2e162c6513ced06f1dd0da35296");
    }
}

#[cfg(test)]
mod index_tests {
    use super::*;

    #[test]
    fn test_index_key_for_adapter_address() {
        let addr = hex_literal::hex!("d91E80cF2E7be2e162c6513ceD06f1dD0dA35296");
        assert_eq!(
            index_key_for_address(&addr),
            Some("evt_addr:0xd91e80cf2e7be2e162c6513ced06f1dd0da35296".to_string())
        );
    }

    #[test]
    fn test_index_key_for_unrelated_address() {
        let addr = hex_literal::hex!("00000000000000000000000000000000000000ff");
        assert_eq!(index_key_for_address(&addr), None);
    }
}
