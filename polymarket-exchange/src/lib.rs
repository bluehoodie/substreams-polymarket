pub mod abi;
#[allow(dead_code, clippy::all)]
pub mod pb;

use substreams::errors::Error;
use substreams_ethereum::pb::eth::v2 as eth;

use pb::polymarket::exchange::v1 as proto;
use polymarket_substreams_common::{
    bigint_to_string, bigint_to_u32, build_tx_context, format_address,
    CTF_EXCHANGE as CTF_EXCHANGE_CONTRACT_ADDRESS,
};

#[substreams::handlers::map]
pub fn map_exchange_events(blk: eth::Block) -> Result<proto::ExchangeEvents, Error> {
    use abi::ctf_exchange::events::*;

    let mut events = proto::ExchangeEvents::default();

    for log in blk.logs() {
        if !is_exchange_contract(log.log) {
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
    use abi::ctf_exchange::events::*;

    let mut events = proto::FeeEvents::default();

    for log in blk.logs() {
        if !is_exchange_contract(log.log) {
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
    use abi::ctf_exchange::events::*;

    let mut events = proto::AdminEvents::default();

    for log in blk.logs() {
        if !is_exchange_contract(log.log) {
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
    use abi::ctf_exchange::events::*;

    let mut events = proto::PauseEvents::default();

    for log in blk.logs() {
        if !is_exchange_contract(log.log) {
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
    use abi::ctf_exchange::events::*;

    let mut events = proto::OrderApprovalEvents::default();

    for log in blk.logs() {
        if !is_exchange_contract(log.log) {
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
    use abi::ctf_exchange::events::*;

    let mut exchange_events = proto::ExchangeEvents::default();
    let mut fee_events = proto::FeeEvents::default();
    let mut admin_events = proto::AdminEvents::default();
    let mut pause_events = proto::PauseEvents::default();
    let mut approval_events = proto::OrderApprovalEvents::default();

    for log in blk.logs() {
        if !is_exchange_contract(log.log) {
            continue;
        }

        if OrderFilled::match_log(log.log) {
            if let Ok(event) = OrderFilled::decode(log.log) {
                exchange_events.order_filled.push(proto::OrderFilled {
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
                exchange_events.orders_matched.push(proto::OrdersMatched {
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
        exchange_events: if !exchange_events.order_filled.is_empty()
            || !exchange_events.orders_matched.is_empty()
        {
            Some(exchange_events)
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
fn is_exchange_contract(log: &eth::Log) -> bool {
    log.address == CTF_EXCHANGE_CONTRACT_ADDRESS
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
    fn test_bigint_zero_to_string() {
        let zero = substreams::scalar::BigInt::from(0u64);
        assert_eq!(zero.to_string(), "0");
    }

    #[test]
    fn test_is_exchange_contract_matching_address() {
        let log = eth::Log {
            address: hex_literal::hex!("E111180000d2663C0091e4f400237545B87B996B").to_vec(),
            ..Default::default()
        };
        assert!(is_exchange_contract(&log));
    }

    #[test]
    fn test_is_exchange_contract_non_matching_address() {
        let log = eth::Log {
            address: hex_literal::hex!("0000000000000000000000000000000000000000").to_vec(),
            ..Default::default()
        };
        assert!(!is_exchange_contract(&log));
    }

    #[test]
    fn test_exchange_contract_address_is_20_bytes() {
        assert_eq!(CTF_EXCHANGE_CONTRACT_ADDRESS.len(), 20);
    }

    #[test]
    fn test_format_address() {
        let bytes = hex_literal::hex!("E111180000d2663C0091e4f400237545B87B996B");
        let result = format_address(&bytes);
        assert_eq!(result, "0xe111180000d2663c0091e4f400237545b87b996b");
    }

    #[test]
    fn test_order_filled_decodes_valid_log() {
        use crate::abi::ctf_exchange::events::OrderFilled;

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
            address: CTF_EXCHANGE_CONTRACT_ADDRESS.to_vec(),
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
        use crate::abi::ctf_exchange::events::OrderFilled;

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
            address: CTF_EXCHANGE_CONTRACT_ADDRESS.to_vec(),
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
        use crate::abi::ctf_exchange::events::FeeCharged;

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
            address: CTF_EXCHANGE_CONTRACT_ADDRESS.to_vec(),
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
mod handler_tests {
    use super::*;
    use substreams_ethereum::pb::eth::v2 as eth;

    /// Wraps a single log in a minimal successful-transaction block whose header
    /// carries a timestamp, so `map_all_events` can build a transaction context
    /// (which unwraps `blk.header.timestamp`) without panicking.
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

    fn order_filled_log() -> eth::Log {
        let topic0 =
            hex_literal::hex!("d543adfd945773f1a62f74f0ee55a5e3b9b1a28262980ba90b1a89f2ea84d8ee")
                .to_vec();
        let order_hash = [0x11u8; 32];
        let maker = hex_literal::hex!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
        let taker = hex_literal::hex!("BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB");
        let mut maker_t = [0u8; 32];
        maker_t[12..].copy_from_slice(&maker);
        let mut taker_t = [0u8; 32];
        taker_t[12..].copy_from_slice(&taker);
        let word = |v: u64| {
            let mut w = [0u8; 32];
            w[24..].copy_from_slice(&v.to_be_bytes());
            w
        };
        let mut data = Vec::new();
        data.extend_from_slice(&word(1)); // side
        data.extend_from_slice(&word(0x1234)); // token_id
        data.extend_from_slice(&word(1_000_000)); // maker_amount_filled
        data.extend_from_slice(&word(2_000_000)); // taker_amount_filled
        data.extend_from_slice(&word(500)); // fee
        data.extend_from_slice(&[0xAAu8; 32]); // builder
        data.extend_from_slice(&[0xBBu8; 32]); // metadata
        eth::Log {
            address: CTF_EXCHANGE_CONTRACT_ADDRESS.to_vec(),
            topics: vec![topic0, order_hash.to_vec(), maker_t.to_vec(), taker_t.to_vec()],
            data,
            ..Default::default()
        }
    }

    #[test]
    fn map_all_events_classifies_order_filled() {
        let out = __impl_map_all_events(block_with_log(order_filled_log())).expect("handler must not err");
        let ex = out
            .exchange_events
            .expect("OrderFilled from the exchange contract must land in exchange_events");
        assert_eq!(ex.order_filled.len(), 1, "exactly one OrderFilled expected");
        assert_eq!(
            ex.order_filled[0].maker,
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        );
        // Classification is exclusive: an OrderFilled must not leak into other buckets.
        assert!(out.fee_events.is_none());
        assert!(out.admin_events.is_none());
        assert!(out.pause_events.is_none());
        assert!(out.order_approval_events.is_none());
    }

    #[test]
    fn map_all_events_empty_block_ok() {
        let out = __impl_map_all_events(eth::Block::default()).expect("empty block must not panic");
        assert!(out.exchange_events.is_none());
        assert!(out.fee_events.is_none());
        assert!(out.admin_events.is_none());
        assert!(out.pause_events.is_none());
        assert!(out.order_approval_events.is_none());
    }
}
