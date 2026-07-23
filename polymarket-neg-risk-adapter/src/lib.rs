pub mod abi;
#[allow(dead_code, clippy::all)]
pub mod pb;

use substreams::errors::Error;
use substreams_ethereum::pb::eth::v2 as eth;

use pb::polymarket::neg_risk_adapter::v1 as proto;
use polymarket_substreams_common::{
    bigint_to_string, build_tx_context, format_address,
    NEG_RISK_ADAPTER as NEG_RISK_ADAPTER_CONTRACT_ADDRESS,
};

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
                market_events
                    .question_prepared
                    .push(proto::QuestionPrepared {
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
                trading_events
                    .positions_converted
                    .push(proto::PositionsConverted {
                        stakeholder: format_address(&event.stakeholder),
                        market_id: event.market_id.to_vec(),
                        index_set: bigint_to_string(&event.index_set),
                        amount: bigint_to_string(&event.amount),
                        tx: Some(build_transaction_context(&blk, &log)),
                    });
            }
        } else if PayoutRedemption::match_log(log.log) {
            if let Ok(event) = PayoutRedemption::decode(log.log) {
                trading_events
                    .payout_redemption
                    .push(proto::PayoutRedemption {
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
mod handler_tests {
    use super::*;
    use substreams_ethereum::pb::eth::v2 as eth;

    fn block_with_log(log: eth::Log) -> eth::Block {
        use substreams_ethereum::pb::eth::v2::{BlockHeader, TransactionReceipt, TransactionTrace};
        eth::Block {
            number: 42,
            header: Some(BlockHeader {
                timestamp: Some(prost_types::Timestamp {
                    seconds: 1_700_000_000,
                    nanos: 0,
                }),
                ..Default::default()
            }),
            transaction_traces: vec![TransactionTrace {
                hash: vec![0xabu8; 32],
                status: 1,
                receipt: Some(TransactionReceipt {
                    logs: vec![log],
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn outcome_reported_log() -> eth::Log {
        let topic0 =
            hex_literal::hex!("9e9fa7fd355160bd4cd3f22d4333519354beff1f5689bde87f2c5e63d8d484b2")
                .to_vec();
        let market_id = [0x77u8; 32];
        let question_id = [0x88u8; 32];
        let mut outcome = [0u8; 32];
        outcome[31] = 1; // bool true
        eth::Log {
            address: NEG_RISK_ADAPTER_CONTRACT_ADDRESS.to_vec(),
            topics: vec![topic0, market_id.to_vec(), question_id.to_vec()],
            data: outcome.to_vec(),
            ..Default::default()
        }
    }

    #[test]
    fn map_all_events_classifies_outcome_reported() {
        let out =
            __impl_map_all_events(block_with_log(outcome_reported_log())).expect("handler must not err");
        let market = out
            .market_events
            .expect("OutcomeReported must land in market_events");
        assert_eq!(market.outcome_reported.len(), 1);
        assert!(market.outcome_reported[0].outcome, "outcome bool must decode true");
        assert_eq!(market.outcome_reported[0].market_id, [0x77u8; 32].to_vec());
        // A market event must not be classified as trading/admin.
        assert!(out.trading_events.is_none());
        assert!(out.admin_events.is_none());
    }

    #[test]
    fn map_all_events_empty_block_ok() {
        let out = __impl_map_all_events(eth::Block::default()).expect("empty block must not panic");
        assert!(out.market_events.is_none());
        assert!(out.trading_events.is_none());
        assert!(out.admin_events.is_none());
    }
}
