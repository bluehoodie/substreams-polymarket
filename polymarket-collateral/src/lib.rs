pub mod abi;
#[allow(dead_code, clippy::all)]
pub mod pb;

use substreams::errors::Error;
use substreams_ethereum::pb::eth::v2 as eth;

use pb::polymarket::collateral::v1 as proto;
use polymarket_substreams_common::{
    bigint_to_string, build_tx_context, format_address,
    CTF_COLLATERAL_ADAPTER as CTF_COLLATERAL_ADAPTER_ADDRESS,
    NEG_RISK_CTF_COLLATERAL_ADAPTER as NEG_RISK_CTF_COLLATERAL_ADAPTER_ADDRESS,
    PUSD as PUSD_CONTRACT_ADDRESS,
};

#[substreams::handlers::map]
pub fn map_pusd_events(blk: eth::Block) -> Result<proto::PusdEvents, Error> {
    use abi::p_usd::events::*;

    let mut events = proto::PusdEvents::default();

    for log in blk.logs() {
        if !is_pusd_contract(log.log) {
            continue;
        }

        if Transfer::match_log(log.log) {
            if let Ok(event) = Transfer::decode(log.log) {
                events.transfer.push(proto::Transfer {
                    from: format_address(&event.from),
                    to: format_address(&event.to),
                    amount: bigint_to_string(&event.value),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if Wrapped::match_log(log.log) {
            if let Ok(event) = Wrapped::decode(log.log) {
                events.wrapped.push(proto::Wrapped {
                    caller: format_address(&event.caller),
                    asset: format_address(&event.asset),
                    to: format_address(&event.to),
                    amount: bigint_to_string(&event.amount),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if Unwrapped::match_log(log.log) {
            if let Ok(event) = Unwrapped::decode(log.log) {
                events.unwrapped.push(proto::Unwrapped {
                    caller: format_address(&event.caller),
                    asset: format_address(&event.asset),
                    to: format_address(&event.to),
                    amount: bigint_to_string(&event.amount),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn map_ctf_adapter_events(blk: eth::Block) -> Result<proto::CtfAdapterEvents, Error> {
    use abi::ctf_collateral_adapter::events::*;

    let mut events = proto::CtfAdapterEvents::default();

    for log in blk.logs() {
        if !is_ctf_adapter_contract(log.log) {
            continue;
        }

        if PositionSplit::match_log(log.log) {
            if let Ok(event) = PositionSplit::decode(log.log) {
                events.position_split.push(proto::PositionSplit {
                    initiator: format_address(&event.initiator),
                    condition_id: event.condition_id.to_vec(),
                    amount: bigint_to_string(&event.amount),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if PositionsMerged::match_log(log.log) {
            if let Ok(event) = PositionsMerged::decode(log.log) {
                events.positions_merged.push(proto::PositionsMerged {
                    initiator: format_address(&event.initiator),
                    condition_id: event.condition_id.to_vec(),
                    amount: bigint_to_string(&event.amount),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if PositionsRedeemed::match_log(log.log) {
            if let Ok(event) = PositionsRedeemed::decode(log.log) {
                events.positions_redeemed.push(proto::PositionsRedeemed {
                    initiator: format_address(&event.initiator),
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
pub fn map_neg_risk_ctf_adapter_events(
    blk: eth::Block,
) -> Result<proto::NegRiskCtfAdapterEvents, Error> {
    use abi::neg_risk_ctf_collateral_adapter::events::*;

    let mut events = proto::NegRiskCtfAdapterEvents::default();

    for log in blk.logs() {
        if !is_neg_risk_ctf_adapter_contract(log.log) {
            continue;
        }

        if PositionSplit::match_log(log.log) {
            if let Ok(event) = PositionSplit::decode(log.log) {
                events.position_split.push(proto::PositionSplit {
                    initiator: format_address(&event.initiator),
                    condition_id: event.condition_id.to_vec(),
                    amount: bigint_to_string(&event.amount),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if PositionsMerged::match_log(log.log) {
            if let Ok(event) = PositionsMerged::decode(log.log) {
                events.positions_merged.push(proto::PositionsMerged {
                    initiator: format_address(&event.initiator),
                    condition_id: event.condition_id.to_vec(),
                    amount: bigint_to_string(&event.amount),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if PositionsRedeemed::match_log(log.log) {
            if let Ok(event) = PositionsRedeemed::decode(log.log) {
                events.positions_redeemed.push(proto::PositionsRedeemed {
                    initiator: format_address(&event.initiator),
                    condition_id: event.condition_id.to_vec(),
                    amounts: event.amounts.iter().map(bigint_to_string).collect(),
                    payout: bigint_to_string(&event.payout),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        } else if PositionsConverted::match_log(log.log) {
            if let Ok(event) = PositionsConverted::decode(log.log) {
                events.positions_converted.push(proto::PositionsConverted {
                    initiator: format_address(&event.initiator),
                    market_id: event.market_id.to_vec(),
                    index_set: bigint_to_string(&event.index_set),
                    amount: bigint_to_string(&event.amount),
                    amount_out: bigint_to_string(&event.amount_out),
                    tx: Some(build_transaction_context(&blk, &log)),
                });
            }
        }
    }

    Ok(events)
}

#[substreams::handlers::map]
pub fn map_all_events(blk: eth::Block) -> Result<proto::AllEvents, Error> {
    use abi::ctf_collateral_adapter::events as ctf_events;
    use abi::neg_risk_ctf_collateral_adapter::events as neg_risk_events;
    use abi::p_usd::events as pusd_events;

    let mut pusd = proto::PusdEvents::default();
    let mut ctf = proto::CtfAdapterEvents::default();
    let mut neg_risk = proto::NegRiskCtfAdapterEvents::default();

    // Contract-first dispatch: match the log address once, then branch per event.
    // Intentional for this multi-contract package — avoids redundant per-event
    // address checks when a log belongs to none of the matched contracts.
    for log in blk.logs() {
        let addr = &log.log.address;

        if addr == &PUSD_CONTRACT_ADDRESS[..] {
            if pusd_events::Transfer::match_log(log.log) {
                if let Ok(event) = pusd_events::Transfer::decode(log.log) {
                    pusd.transfer.push(proto::Transfer {
                        from: format_address(&event.from),
                        to: format_address(&event.to),
                        amount: bigint_to_string(&event.value),
                        tx: Some(build_transaction_context(&blk, &log)),
                    });
                }
            } else if pusd_events::Wrapped::match_log(log.log) {
                if let Ok(event) = pusd_events::Wrapped::decode(log.log) {
                    pusd.wrapped.push(proto::Wrapped {
                        caller: format_address(&event.caller),
                        asset: format_address(&event.asset),
                        to: format_address(&event.to),
                        amount: bigint_to_string(&event.amount),
                        tx: Some(build_transaction_context(&blk, &log)),
                    });
                }
            } else if pusd_events::Unwrapped::match_log(log.log) {
                if let Ok(event) = pusd_events::Unwrapped::decode(log.log) {
                    pusd.unwrapped.push(proto::Unwrapped {
                        caller: format_address(&event.caller),
                        asset: format_address(&event.asset),
                        to: format_address(&event.to),
                        amount: bigint_to_string(&event.amount),
                        tx: Some(build_transaction_context(&blk, &log)),
                    });
                }
            }
        } else if addr == &CTF_COLLATERAL_ADAPTER_ADDRESS[..] {
            if ctf_events::PositionSplit::match_log(log.log) {
                if let Ok(event) = ctf_events::PositionSplit::decode(log.log) {
                    ctf.position_split.push(proto::PositionSplit {
                        initiator: format_address(&event.initiator),
                        condition_id: event.condition_id.to_vec(),
                        amount: bigint_to_string(&event.amount),
                        tx: Some(build_transaction_context(&blk, &log)),
                    });
                }
            } else if ctf_events::PositionsMerged::match_log(log.log) {
                if let Ok(event) = ctf_events::PositionsMerged::decode(log.log) {
                    ctf.positions_merged.push(proto::PositionsMerged {
                        initiator: format_address(&event.initiator),
                        condition_id: event.condition_id.to_vec(),
                        amount: bigint_to_string(&event.amount),
                        tx: Some(build_transaction_context(&blk, &log)),
                    });
                }
            } else if ctf_events::PositionsRedeemed::match_log(log.log) {
                if let Ok(event) = ctf_events::PositionsRedeemed::decode(log.log) {
                    ctf.positions_redeemed.push(proto::PositionsRedeemed {
                        initiator: format_address(&event.initiator),
                        condition_id: event.condition_id.to_vec(),
                        amounts: event.amounts.iter().map(bigint_to_string).collect(),
                        payout: bigint_to_string(&event.payout),
                        tx: Some(build_transaction_context(&blk, &log)),
                    });
                }
            }
        } else if addr == &NEG_RISK_CTF_COLLATERAL_ADAPTER_ADDRESS[..] {
            if neg_risk_events::PositionSplit::match_log(log.log) {
                if let Ok(event) = neg_risk_events::PositionSplit::decode(log.log) {
                    neg_risk.position_split.push(proto::PositionSplit {
                        initiator: format_address(&event.initiator),
                        condition_id: event.condition_id.to_vec(),
                        amount: bigint_to_string(&event.amount),
                        tx: Some(build_transaction_context(&blk, &log)),
                    });
                }
            } else if neg_risk_events::PositionsMerged::match_log(log.log) {
                if let Ok(event) = neg_risk_events::PositionsMerged::decode(log.log) {
                    neg_risk.positions_merged.push(proto::PositionsMerged {
                        initiator: format_address(&event.initiator),
                        condition_id: event.condition_id.to_vec(),
                        amount: bigint_to_string(&event.amount),
                        tx: Some(build_transaction_context(&blk, &log)),
                    });
                }
            } else if neg_risk_events::PositionsRedeemed::match_log(log.log) {
                if let Ok(event) = neg_risk_events::PositionsRedeemed::decode(log.log) {
                    neg_risk.positions_redeemed.push(proto::PositionsRedeemed {
                        initiator: format_address(&event.initiator),
                        condition_id: event.condition_id.to_vec(),
                        amounts: event.amounts.iter().map(bigint_to_string).collect(),
                        payout: bigint_to_string(&event.payout),
                        tx: Some(build_transaction_context(&blk, &log)),
                    });
                }
            } else if neg_risk_events::PositionsConverted::match_log(log.log) {
                if let Ok(event) = neg_risk_events::PositionsConverted::decode(log.log) {
                    neg_risk
                        .positions_converted
                        .push(proto::PositionsConverted {
                            initiator: format_address(&event.initiator),
                            market_id: event.market_id.to_vec(),
                            index_set: bigint_to_string(&event.index_set),
                            amount: bigint_to_string(&event.amount),
                            amount_out: bigint_to_string(&event.amount_out),
                            tx: Some(build_transaction_context(&blk, &log)),
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
            Some(pusd)
        } else {
            None
        },
        ctf_adapter_events: if !ctf.position_split.is_empty()
            || !ctf.positions_merged.is_empty()
            || !ctf.positions_redeemed.is_empty()
        {
            Some(ctf)
        } else {
            None
        },
        neg_risk_ctf_adapter_events: if !neg_risk.position_split.is_empty()
            || !neg_risk.positions_merged.is_empty()
            || !neg_risk.positions_redeemed.is_empty()
            || !neg_risk.positions_converted.is_empty()
        {
            Some(neg_risk)
        } else {
            None
        },
    })
}

#[inline]
fn is_pusd_contract(log: &eth::Log) -> bool {
    log.address == PUSD_CONTRACT_ADDRESS
}

#[inline]
fn is_ctf_adapter_contract(log: &eth::Log) -> bool {
    log.address == CTF_COLLATERAL_ADAPTER_ADDRESS
}

#[inline]
fn is_neg_risk_ctf_adapter_contract(log: &eth::Log) -> bool {
    log.address == NEG_RISK_CTF_COLLATERAL_ADAPTER_ADDRESS
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
        assert!(is_pusd_contract(&log));
    }

    #[test]
    fn test_is_pusd_contract_non_matching() {
        let log = eth::Log {
            address: hex_literal::hex!("0000000000000000000000000000000000000000").to_vec(),
            ..Default::default()
        };
        assert!(!is_pusd_contract(&log));
    }

    #[test]
    fn test_is_ctf_adapter_contract_matching() {
        let log = eth::Log {
            address: hex_literal::hex!("AdA100Db00Ca00073811820692005400218FcE1f").to_vec(),
            ..Default::default()
        };
        assert!(is_ctf_adapter_contract(&log));
    }

    #[test]
    fn test_is_ctf_adapter_contract_non_matching() {
        let log = eth::Log {
            address: hex_literal::hex!("0000000000000000000000000000000000000000").to_vec(),
            ..Default::default()
        };
        assert!(!is_ctf_adapter_contract(&log));
    }

    #[test]
    fn test_is_neg_risk_ctf_adapter_contract_matching() {
        let log = eth::Log {
            address: hex_literal::hex!("adA2005600Dec949baf300f4C6120000bDB6eAab").to_vec(),
            ..Default::default()
        };
        assert!(is_neg_risk_ctf_adapter_contract(&log));
    }

    #[test]
    fn test_is_neg_risk_ctf_adapter_contract_non_matching() {
        let log = eth::Log {
            address: hex_literal::hex!("0000000000000000000000000000000000000000").to_vec(),
            ..Default::default()
        };
        assert!(!is_neg_risk_ctf_adapter_contract(&log));
    }

    #[test]
    fn test_format_address() {
        let bytes = hex_literal::hex!("C011a7E12a19f7B1f670d46F03B03f3342E82DFB");
        let result = format_address(&bytes);
        assert_eq!(result, "0xc011a7e12a19f7b1f670d46f03b03f3342e82dfb");
    }
}
