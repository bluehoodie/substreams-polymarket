#[allow(dead_code, clippy::all)]
pub mod pb;
pub mod abi;

use substreams::errors::Error;
use substreams_ethereum::pb::eth::v2 as eth;

use pb::polymarket::neg_risk_ctf::v1 as proto;
use polymarket_substreams_common::{bigint_to_string, bigint_to_u32, build_tx_context, format_address};

const NEG_RISK_CTF_CONTRACT_ADDRESS: [u8; 20] = hex_literal::hex!("e2222d279d744050d28e00520010520000310F59");

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
                    token_id: bigint_to_string(&event.token_id),
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
                events.user_pause_block_interval_updated.push(proto::UserPauseBlockIntervalUpdated {
                    old_interval: bigint_to_string(&event.old_interval),
                    new_interval: bigint_to_string(&event.new_interval),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
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
                events.order_preapproval_invalidated.push(proto::OrderPreapprovalInvalidated {
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
                    token_id: bigint_to_string(&event.token_id),
                    amount: bigint_to_string(&event.amount),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if FeeReceiverUpdated::match_log(log.log) {
            if let Ok(event) = FeeReceiverUpdated::decode(log.log) {
                fee_events.fee_receiver_updated.push(proto::FeeReceiverUpdated {
                    fee_receiver: format_address(&event.fee_receiver),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if MaxFeeRateUpdated::match_log(log.log) {
            if let Ok(event) = MaxFeeRateUpdated::decode(log.log) {
                fee_events.max_fee_rate_updated.push(proto::MaxFeeRateUpdated {
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
                pause_events.user_pause_block_interval_updated.push(proto::UserPauseBlockIntervalUpdated {
                    old_interval: bigint_to_string(&event.old_interval),
                    new_interval: bigint_to_string(&event.new_interval),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if OrderPreapproved::match_log(log.log) {
            if let Ok(event) = OrderPreapproved::decode(log.log) {
                approval_events.order_preapproved.push(proto::OrderPreapproved {
                    order_hash: event.order_hash.to_vec(),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if OrderPreapprovalInvalidated::match_log(log.log) {
            if let Ok(event) = OrderPreapprovalInvalidated::decode(log.log) {
                approval_events.order_preapproval_invalidated.push(proto::OrderPreapprovalInvalidated {
                    order_hash: event.order_hash.to_vec(),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
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
}
