#[allow(dead_code, clippy::all)]
pub mod pb;
pub mod abi;

use substreams::errors::Error;
use substreams_ethereum::pb::eth::v2 as eth;

use pb::polymarket::neg_risk_adapter::v1 as proto;

const NEG_RISK_ADAPTER_CONTRACT_ADDRESS: [u8; 20] = hex_literal::hex!("d91E80cF2E7be2e162c6513ceD06f1dD0dA35296");

#[substreams::handlers::map]
fn map_market_events(blk: eth::Block) -> Result<proto::MarketEvents, Error> {
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
fn map_trading_events(blk: eth::Block) -> Result<proto::TradingEvents, Error> {
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
                    amounts: event.amounts.iter().map(|a| bigint_to_string(a)).collect(),
                    payout: bigint_to_string(&event.payout),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
fn map_admin_events(blk: eth::Block) -> Result<proto::AdminEvents, Error> {
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
                    amounts: event.amounts.iter().map(|a| bigint_to_string(a)).collect(),
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
fn bigint_to_string(bigint: &substreams::scalar::BigInt) -> String {
    let s = bigint.to_string();
    if s.is_empty() { "0".to_string() } else { s }
}

#[inline]
fn is_adapter_contract(log: &eth::Log) -> bool {
    log.address == NEG_RISK_ADAPTER_CONTRACT_ADDRESS
}

#[inline]
fn format_address(bytes: &[u8]) -> String {
    format!("0x{}", hex::encode(bytes))
}

#[inline]
fn build_transaction_context(blk: &eth::Block, log: &substreams_ethereum::block_view::LogView) -> proto::TransactionContext {
    proto::TransactionContext {
        tx_hash: format_address(&log.receipt.transaction.hash),
        log_index: log.log.block_index as u64,
        block_number: blk.number,
        timestamp: blk.timestamp_seconds(),
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
