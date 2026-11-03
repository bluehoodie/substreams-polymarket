// The `#[substreams::handlers::map]` macro generates an `extern "C"` shim that rebuilds
// lazy `&eth::BlockLazyView<'_>` handler inputs from a raw `*mut u8`. The macro drops
// the annotated fn's attributes, so a fn-scoped allow cannot reach the generated
// shim; `not_unsafe_ptr_arg_deref` must be allowed at crate scope. The unsafe deref
// lives entirely in macro-generated code; our handlers are safe.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

#[allow(dead_code, clippy::all)]
pub mod pb;

use buffa::MessageField;
use substreams::errors::Error;
use substreams_ethereum::pb::eth::v2 as eth;

use pb::polymarket::collateral::v1 as proto;
use polymarket_substreams_common::{
    bigint_to_string, block_logs, format_address, tx_context, BlockLog, BlockLogs,
    CTF_COLLATERAL_ADAPTER as CTF_COLLATERAL_ADAPTER_ADDRESS,
    NEG_RISK_CTF_COLLATERAL_ADAPTER as NEG_RISK_CTF_COLLATERAL_ADAPTER_ADDRESS,
    PUSD as PUSD_CONTRACT_ADDRESS,
};

#[substreams::handlers::map]
pub fn map_pusd_events(blk: &eth::BlockLazyView<'_>) -> Result<proto::PusdEvents, Error> {
    substreams::skip_empty_output();
    use polymarket_substreams_common::abi::p_usd::events::*;

    let mut events = proto::PusdEvents::default();

    let blk = block_logs(blk, is_pusd_contract)?;
    for log in &blk.logs {
        if Transfer::match_log(&log.log) {
            if let Ok(event) = Transfer::decode(&log.log) {
                events.transfer.push(proto::Transfer {
                    from: format_address(&event.from),
                    to: format_address(&event.to),
                    amount: bigint_to_string(&event.value),
                    tx: build_transaction_context(&blk, log).into(),
                });
            }
        } else if Wrapped::match_log(&log.log) {
            if let Ok(event) = Wrapped::decode(&log.log) {
                events.wrapped.push(proto::Wrapped {
                    caller: format_address(&event.caller),
                    asset: format_address(&event.asset),
                    to: format_address(&event.to),
                    amount: bigint_to_string(&event.amount),
                    tx: build_transaction_context(&blk, log).into(),
                });
            }
        } else if Unwrapped::match_log(&log.log) {
            if let Ok(event) = Unwrapped::decode(&log.log) {
                events.unwrapped.push(proto::Unwrapped {
                    caller: format_address(&event.caller),
                    asset: format_address(&event.asset),
                    to: format_address(&event.to),
                    amount: bigint_to_string(&event.amount),
                    tx: build_transaction_context(&blk, log).into(),
                });
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn map_ctf_adapter_events(
    blk: &eth::BlockLazyView<'_>,
) -> Result<proto::CtfAdapterEvents, Error> {
    substreams::skip_empty_output();
    use polymarket_substreams_common::abi::ctf_collateral_adapter::events::*;

    let mut events = proto::CtfAdapterEvents::default();

    let blk = block_logs(blk, is_ctf_adapter_contract)?;
    for log in &blk.logs {
        if PositionSplit::match_log(&log.log) {
            if let Ok(event) = PositionSplit::decode(&log.log) {
                events.position_split.push(proto::PositionSplit {
                    initiator: format_address(&event.initiator),
                    condition_id: event.condition_id.to_vec(),
                    amount: bigint_to_string(&event.amount),
                    tx: build_transaction_context(&blk, log).into(),
                });
            }
        } else if PositionsMerged::match_log(&log.log) {
            if let Ok(event) = PositionsMerged::decode(&log.log) {
                events.positions_merged.push(proto::PositionsMerged {
                    initiator: format_address(&event.initiator),
                    condition_id: event.condition_id.to_vec(),
                    amount: bigint_to_string(&event.amount),
                    tx: build_transaction_context(&blk, log).into(),
                });
            }
        } else if PositionsRedeemed::match_log(&log.log) {
            if let Ok(event) = PositionsRedeemed::decode(&log.log) {
                events.positions_redeemed.push(proto::PositionsRedeemed {
                    initiator: format_address(&event.initiator),
                    condition_id: event.condition_id.to_vec(),
                    amounts: event.amounts.iter().map(bigint_to_string).collect(),
                    payout: bigint_to_string(&event.payout),
                    tx: build_transaction_context(&blk, log).into(),
                });
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn map_neg_risk_ctf_adapter_events(
    blk: &eth::BlockLazyView<'_>,
) -> Result<proto::NegRiskCtfAdapterEvents, Error> {
    substreams::skip_empty_output();
    use polymarket_substreams_common::abi::neg_risk_ctf_collateral_adapter::events::*;

    let mut events = proto::NegRiskCtfAdapterEvents::default();

    let blk = block_logs(blk, is_neg_risk_ctf_adapter_contract)?;
    for log in &blk.logs {
        if PositionSplit::match_log(&log.log) {
            if let Ok(event) = PositionSplit::decode(&log.log) {
                events.position_split.push(proto::PositionSplit {
                    initiator: format_address(&event.initiator),
                    condition_id: event.condition_id.to_vec(),
                    amount: bigint_to_string(&event.amount),
                    tx: build_transaction_context(&blk, log).into(),
                });
            }
        } else if PositionsMerged::match_log(&log.log) {
            if let Ok(event) = PositionsMerged::decode(&log.log) {
                events.positions_merged.push(proto::PositionsMerged {
                    initiator: format_address(&event.initiator),
                    condition_id: event.condition_id.to_vec(),
                    amount: bigint_to_string(&event.amount),
                    tx: build_transaction_context(&blk, log).into(),
                });
            }
        } else if PositionsRedeemed::match_log(&log.log) {
            if let Ok(event) = PositionsRedeemed::decode(&log.log) {
                events.positions_redeemed.push(proto::PositionsRedeemed {
                    initiator: format_address(&event.initiator),
                    condition_id: event.condition_id.to_vec(),
                    amounts: event.amounts.iter().map(bigint_to_string).collect(),
                    payout: bigint_to_string(&event.payout),
                    tx: build_transaction_context(&blk, log).into(),
                });
            }
        } else if PositionsConverted::match_log(&log.log) {
            if let Ok(event) = PositionsConverted::decode(&log.log) {
                events.positions_converted.push(proto::PositionsConverted {
                    initiator: format_address(&event.initiator),
                    market_id: event.market_id.to_vec(),
                    index_set: bigint_to_string(&event.index_set),
                    amount: bigint_to_string(&event.amount),
                    amount_out: bigint_to_string(&event.amount_out),
                    tx: build_transaction_context(&blk, log).into(),
                });
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn map_all_events(blk: &eth::BlockLazyView<'_>) -> Result<proto::AllEvents, Error> {
    substreams::skip_empty_output();
    use polymarket_substreams_common::abi::ctf_collateral_adapter::events as ctf_events;
    use polymarket_substreams_common::abi::neg_risk_ctf_collateral_adapter::events as neg_risk_events;
    use polymarket_substreams_common::abi::p_usd::events as pusd_events;

    let mut pusd = proto::PusdEvents::default();
    let mut ctf = proto::CtfAdapterEvents::default();
    let mut neg_risk = proto::NegRiskCtfAdapterEvents::default();

    // Contract-first dispatch: match the log address once, then branch per event.
    // Intentional for this multi-contract package — avoids redundant per-event
    // address checks when a log belongs to none of the matched contracts.
    let blk = block_logs(blk, |a| {
        is_pusd_contract(a) || is_ctf_adapter_contract(a) || is_neg_risk_ctf_adapter_contract(a)
    })?;
    for log in &blk.logs {
        let addr = log.log.address;

        if addr == PUSD_CONTRACT_ADDRESS {
            if pusd_events::Transfer::match_log(&log.log) {
                if let Ok(event) = pusd_events::Transfer::decode(&log.log) {
                    pusd.transfer.push(proto::Transfer {
                        from: format_address(&event.from),
                        to: format_address(&event.to),
                        amount: bigint_to_string(&event.value),
                        tx: build_transaction_context(&blk, log).into(),
                    });
                }
            } else if pusd_events::Wrapped::match_log(&log.log) {
                if let Ok(event) = pusd_events::Wrapped::decode(&log.log) {
                    pusd.wrapped.push(proto::Wrapped {
                        caller: format_address(&event.caller),
                        asset: format_address(&event.asset),
                        to: format_address(&event.to),
                        amount: bigint_to_string(&event.amount),
                        tx: build_transaction_context(&blk, log).into(),
                    });
                }
            } else if pusd_events::Unwrapped::match_log(&log.log) {
                if let Ok(event) = pusd_events::Unwrapped::decode(&log.log) {
                    pusd.unwrapped.push(proto::Unwrapped {
                        caller: format_address(&event.caller),
                        asset: format_address(&event.asset),
                        to: format_address(&event.to),
                        amount: bigint_to_string(&event.amount),
                        tx: build_transaction_context(&blk, log).into(),
                    });
                }
            }
        } else if addr == CTF_COLLATERAL_ADAPTER_ADDRESS {
            if ctf_events::PositionSplit::match_log(&log.log) {
                if let Ok(event) = ctf_events::PositionSplit::decode(&log.log) {
                    ctf.position_split.push(proto::PositionSplit {
                        initiator: format_address(&event.initiator),
                        condition_id: event.condition_id.to_vec(),
                        amount: bigint_to_string(&event.amount),
                        tx: build_transaction_context(&blk, log).into(),
                    });
                }
            } else if ctf_events::PositionsMerged::match_log(&log.log) {
                if let Ok(event) = ctf_events::PositionsMerged::decode(&log.log) {
                    ctf.positions_merged.push(proto::PositionsMerged {
                        initiator: format_address(&event.initiator),
                        condition_id: event.condition_id.to_vec(),
                        amount: bigint_to_string(&event.amount),
                        tx: build_transaction_context(&blk, log).into(),
                    });
                }
            } else if ctf_events::PositionsRedeemed::match_log(&log.log) {
                if let Ok(event) = ctf_events::PositionsRedeemed::decode(&log.log) {
                    ctf.positions_redeemed.push(proto::PositionsRedeemed {
                        initiator: format_address(&event.initiator),
                        condition_id: event.condition_id.to_vec(),
                        amounts: event.amounts.iter().map(bigint_to_string).collect(),
                        payout: bigint_to_string(&event.payout),
                        tx: build_transaction_context(&blk, log).into(),
                    });
                }
            }
        } else if addr == NEG_RISK_CTF_COLLATERAL_ADAPTER_ADDRESS {
            if neg_risk_events::PositionSplit::match_log(&log.log) {
                if let Ok(event) = neg_risk_events::PositionSplit::decode(&log.log) {
                    neg_risk.position_split.push(proto::PositionSplit {
                        initiator: format_address(&event.initiator),
                        condition_id: event.condition_id.to_vec(),
                        amount: bigint_to_string(&event.amount),
                        tx: build_transaction_context(&blk, log).into(),
                    });
                }
            } else if neg_risk_events::PositionsMerged::match_log(&log.log) {
                if let Ok(event) = neg_risk_events::PositionsMerged::decode(&log.log) {
                    neg_risk.positions_merged.push(proto::PositionsMerged {
                        initiator: format_address(&event.initiator),
                        condition_id: event.condition_id.to_vec(),
                        amount: bigint_to_string(&event.amount),
                        tx: build_transaction_context(&blk, log).into(),
                    });
                }
            } else if neg_risk_events::PositionsRedeemed::match_log(&log.log) {
                if let Ok(event) = neg_risk_events::PositionsRedeemed::decode(&log.log) {
                    neg_risk.positions_redeemed.push(proto::PositionsRedeemed {
                        initiator: format_address(&event.initiator),
                        condition_id: event.condition_id.to_vec(),
                        amounts: event.amounts.iter().map(bigint_to_string).collect(),
                        payout: bigint_to_string(&event.payout),
                        tx: build_transaction_context(&blk, log).into(),
                    });
                }
            } else if neg_risk_events::PositionsConverted::match_log(&log.log) {
                if let Ok(event) = neg_risk_events::PositionsConverted::decode(&log.log) {
                    neg_risk
                        .positions_converted
                        .push(proto::PositionsConverted {
                            initiator: format_address(&event.initiator),
                            market_id: event.market_id.to_vec(),
                            index_set: bigint_to_string(&event.index_set),
                            amount: bigint_to_string(&event.amount),
                            amount_out: bigint_to_string(&event.amount_out),
                            tx: build_transaction_context(&blk, log).into(),
                        });
                }
            }
        }
    }

    Ok(proto::AllEvents {
        pusd_events: if !pusd.transfer.is_empty()
            || !pusd.wrapped.is_empty()
            || !pusd.unwrapped.is_empty()
        {
            pusd.into()
        } else {
            MessageField::none()
        },
        ctf_adapter_events: if !ctf.position_split.is_empty()
            || !ctf.positions_merged.is_empty()
            || !ctf.positions_redeemed.is_empty()
        {
            ctf.into()
        } else {
            MessageField::none()
        },
        neg_risk_ctf_adapter_events: if !neg_risk.position_split.is_empty()
            || !neg_risk.positions_merged.is_empty()
            || !neg_risk.positions_redeemed.is_empty()
            || !neg_risk.positions_converted.is_empty()
        {
            neg_risk.into()
        } else {
            MessageField::none()
        },
    })
}

#[inline]
fn is_pusd_contract(address: &[u8]) -> bool {
    address == PUSD_CONTRACT_ADDRESS
}

#[inline]
fn is_ctf_adapter_contract(address: &[u8]) -> bool {
    address == CTF_COLLATERAL_ADAPTER_ADDRESS
}

#[inline]
fn is_neg_risk_ctf_adapter_contract(address: &[u8]) -> bool {
    address == NEG_RISK_CTF_COLLATERAL_ADAPTER_ADDRESS
}

#[inline]
fn build_transaction_context(blk: &BlockLogs, log: &BlockLog) -> proto::TransactionContext {
    let ctx = tx_context(log.tx_hash, &log.log, blk.number, blk.timestamp);
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
    fn test_pusd_address_is_20_bytes() {
        assert_eq!(PUSD_CONTRACT_ADDRESS.len(), 20);
    }

    #[test]
    fn test_ctf_adapter_address_is_20_bytes() {
        assert_eq!(CTF_COLLATERAL_ADAPTER_ADDRESS.len(), 20);
    }

    #[test]
    fn test_neg_risk_ctf_adapter_address_is_20_bytes() {
        assert_eq!(NEG_RISK_CTF_COLLATERAL_ADAPTER_ADDRESS.len(), 20);
    }

    #[test]
    fn test_is_pusd_contract_matching() {
        let log = eth::Log {
            address: hex_literal::hex!("C011a7E12a19f7B1f670d46F03B03f3342E82DFB").to_vec(),
            ..Default::default()
        };
        assert!(is_pusd_contract(&log.address));
    }

    #[test]
    fn test_is_pusd_contract_non_matching() {
        let log = eth::Log {
            address: hex_literal::hex!("0000000000000000000000000000000000000000").to_vec(),
            ..Default::default()
        };
        assert!(!is_pusd_contract(&log.address));
    }

    #[test]
    fn test_is_ctf_adapter_contract_matching() {
        let log = eth::Log {
            address: hex_literal::hex!("AdA100Db00Ca00073811820692005400218FcE1f").to_vec(),
            ..Default::default()
        };
        assert!(is_ctf_adapter_contract(&log.address));
    }

    #[test]
    fn test_is_ctf_adapter_contract_non_matching() {
        let log = eth::Log {
            address: hex_literal::hex!("0000000000000000000000000000000000000000").to_vec(),
            ..Default::default()
        };
        assert!(!is_ctf_adapter_contract(&log.address));
    }

    #[test]
    fn test_is_neg_risk_ctf_adapter_contract_matching() {
        let log = eth::Log {
            address: hex_literal::hex!("adA2005600Dec949baf300f4C6120000bDB6eAab").to_vec(),
            ..Default::default()
        };
        assert!(is_neg_risk_ctf_adapter_contract(&log.address));
    }

    #[test]
    fn test_is_neg_risk_ctf_adapter_contract_non_matching() {
        let log = eth::Log {
            address: hex_literal::hex!("0000000000000000000000000000000000000000").to_vec(),
            ..Default::default()
        };
        assert!(!is_neg_risk_ctf_adapter_contract(&log.address));
    }

    #[test]
    fn test_format_address() {
        let bytes = hex_literal::hex!("C011a7E12a19f7B1f670d46F03B03f3342E82DFB");
        let result = format_address(&bytes);
        assert_eq!(result, "0xc011a7e12a19f7b1f670d46f03b03f3342e82dfb");
    }
}

#[cfg(test)]
mod handler_tests {
    use super::*;
    use substreams_ethereum::pb::eth::v2 as eth;

    fn block_with_log(log: eth::Log) -> eth::Block {
        use substreams_ethereum::pb::eth::v2::{
            BlockHeader, TransactionReceipt, TransactionTrace, TransactionTraceStatus,
        };
        eth::Block {
            number: 42,
            header: BlockHeader {
                timestamp: buffa_types::google::protobuf::Timestamp {
                    seconds: 1_700_000_000,
                    nanos: 0,
                    ..Default::default()
                }
                .into(),
                ..Default::default()
            }
            .into(),
            transaction_traces: vec![TransactionTrace {
                hash: vec![0xabu8; 32],
                status: TransactionTraceStatus::Succeeded.into(),
                receipt: TransactionReceipt {
                    logs: vec![log],
                    ..Default::default()
                }
                .into(),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn topic_addr(addr: &[u8; 20]) -> Vec<u8> {
        let mut t = vec![0u8; 12];
        t.extend_from_slice(addr);
        t
    }

    fn pusd_transfer_log() -> eth::Log {
        let topic0 =
            hex_literal::hex!("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef")
                .to_vec();
        let from = [0x22u8; 20];
        let to = [0x33u8; 20];
        let mut value = [0u8; 32];
        value[24..].copy_from_slice(&1_000_000u64.to_be_bytes());
        eth::Log {
            address: PUSD_CONTRACT_ADDRESS.to_vec(),
            topics: vec![topic0, topic_addr(&from), topic_addr(&to)],
            data: value.to_vec(),
            ..Default::default()
        }
    }

    #[test]
    fn map_all_events_classifies_pusd_transfer() {
        let out = polymarket_substreams_common::with_lazy_block(
            &block_with_log(pusd_transfer_log()),
            |blk| substreams::testing::map!(map_all_events(blk)),
        )
        .expect("fixture block encodes")
        .expect("handler must not err");
        let pusd = out
            .pusd_events
            .as_option()
            .expect("Transfer from the pUSD contract must land in pusd_events");
        assert_eq!(pusd.transfer.len(), 1);
        assert_eq!(
            pusd.transfer[0].from,
            "0x2222222222222222222222222222222222222222"
        );
        // A pUSD event must not be classified as an adapter event.
        assert!(!out.ctf_adapter_events.is_set());
        assert!(!out.neg_risk_ctf_adapter_events.is_set());
    }

    #[test]
    fn map_all_events_empty_block_ok() {
        let out = polymarket_substreams_common::with_lazy_block(&eth::Block::default(), |blk| {
            substreams::testing::map!(map_all_events(blk))
        })
        .expect("fixture block encodes")
        .expect("empty block must not panic");
        assert!(!out.pusd_events.is_set());
        assert!(!out.ctf_adapter_events.is_set());
        assert!(!out.neg_risk_ctf_adapter_events.is_set());
    }
}

#[cfg(test)]
mod fixture_tests {
    use super::*;

    /// Real Polygon block 96600000 (captured with `scripts/capture_fixture.sh 96600000`). The
    /// expected counts match `substreams run` of `map_all_events` on the same block.
    #[test]
    fn map_all_events_real_block_96600000() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../testdata/polygon_96600000.binpb.b64"
        );
        let block = polymarket_substreams_common::load_block_b64(path).expect("fixture loads");
        assert_eq!(block.number, 96600000);
        let out = polymarket_substreams_common::with_lazy_block(&block, |blk| {
            substreams::testing::map!(map_all_events(blk))
        })
        .expect("fixture block encodes")
        .expect("handler must not err");
        assert_eq!(
            out.pusd_events.as_option().map_or(0, |e| e.transfer.len()),
            87,
            "pusd_events.transfer"
        );
        assert_eq!(
            out.pusd_events.as_option().map_or(0, |e| e.wrapped.len()),
            3,
            "pusd_events.wrapped"
        );
        assert_eq!(
            out.pusd_events.as_option().map_or(0, |e| e.unwrapped.len()),
            10,
            "pusd_events.unwrapped"
        );
        assert_eq!(
            out.ctf_adapter_events
                .as_option()
                .map_or(0, |e| e.positions_redeemed.len()),
            2,
            "ctf_adapter_events.positions_redeemed"
        );
        assert_eq!(
            out.neg_risk_ctf_adapter_events
                .as_option()
                .map_or(0, |e| e.positions_redeemed.len()),
            1,
            "neg_risk_ctf_adapter_events.positions_redeemed"
        );
        assert_eq!(
            out.pusd_events.as_option().expect("pusd events").transfer[0].amount,
            "342926010"
        );
    }
}
