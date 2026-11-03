//! Observed orders: the full signed orders behind the fills, decoded from the
//! calldata of the exchanges' settlement calls.

use substreams::errors::Error;
use substreams_ethereum::pb::eth::v2 as eth;

use polymarket_substreams_common::orders::{
    decode_settlement, exchange_at, Exchange, SettledOrder,
};
use polymarket_substreams_common::{
    bigint_to_string, bigint_to_u64, block_timestamp_seconds, format_address,
};

use crate::pb::polymarket::trades::v1 as proto;
use crate::{format_price, SIDE_BUY};

/// topic0 of `OrderFilled(bytes32,address,address,uint8,uint256,uint256,uint256,
/// uint256,bytes32,bytes32)`, the same on all three exchanges.
const ORDER_FILLED_TOPIC: [u8; 32] =
    hex_literal::hex!("d543adfd945773f1a62f74f0ee55a5e3b9b1a28262980ba90b1a89f2ea84d8ee");

#[substreams::handlers::map]
pub fn map_orders(blk: &eth::BlockLazyView<'_>) -> Result<proto::ObservedOrders, Error> {
    substreams::skip_empty_output();
    let mut orders = Vec::new();
    // Read the header only once a block has a settlement.
    let mut timestamp = None;
    for tx in blk.transaction_traces.iter() {
        let tx = tx?;
        if tx.status != eth::TransactionTraceStatus::Succeeded {
            continue;
        }
        // Settlements are top-level operator calls today; walking every call also
        // catches one routed through a contract. Calls that reverted are skipped.
        let mut settled: Vec<(&Exchange, Vec<SettledOrder>)> = Vec::new();
        for call in tx.calls.iter() {
            let call = call?;
            if call.state_reverted {
                continue;
            }
            let Some(exchange) = exchange_at(call.address) else {
                continue;
            };
            if let Some(decoded) = decode_settlement(exchange, call.input) {
                settled.push((exchange, decoded));
            }
        }
        if settled.is_empty() {
            continue;
        }
        let fills = order_filled_logs(&tx)?;
        let ts = match timestamp {
            Some(ts) => ts,
            None => *timestamp.insert(block_timestamp_seconds(blk)?),
        };
        let tx_hash = format_address(tx.hash);
        for (exchange, decoded) in settled {
            for order in decoded {
                let log_index = fills
                    .iter()
                    .find(|(address, hash, _)| *address == exchange.address && *hash == order.hash)
                    .map_or(0, |(_, _, index)| *index);
                let ctx = proto::TransactionContext {
                    tx_hash: tx_hash.clone(),
                    log_index,
                    block_number: blk.number,
                    timestamp: ts,
                };
                orders.push(observed(exchange, order, ctx));
            }
        }
    }
    Ok(proto::ObservedOrders { orders })
}

/// `(exchange address, order hash, block-wide log index)` of an OrderFilled log.
type OrderFilledLog = ([u8; 20], [u8; 32], u64);

/// Every OrderFilled in the transaction's receipt.
fn order_filled_logs(
    tx: &eth::__buffa::lazy_view::TransactionTraceLazyView<'_>,
) -> Result<Vec<OrderFilledLog>, Error> {
    let mut out = Vec::new();
    let Some(receipt) = tx.receipt.get()? else {
        return Ok(out);
    };
    for log in receipt.logs.iter() {
        let log = log?;
        let Some(exchange) = exchange_at(log.address) else {
            continue;
        };
        let mut topics = log.topics.iter().copied();
        if topics.next() != Some(&ORDER_FILLED_TOPIC[..]) {
            continue;
        }
        if let Some(Ok(hash)) = topics.next().map(<[u8; 32]>::try_from) {
            out.push((exchange.address, hash, log.block_index as u64));
        }
    }
    Ok(out)
}

pub(crate) fn observed(
    exchange: &Exchange,
    o: SettledOrder,
    tx: proto::TransactionContext,
) -> proto::ObservedOrder {
    let (collateral, shares) = if o.side == SIDE_BUY {
        (&o.maker_amount, &o.taker_amount)
    } else {
        (&o.taker_amount, &o.maker_amount)
    };
    proto::ObservedOrder {
        order_hash: format_address(&o.hash),
        exchange: exchange.name.to_owned(),
        maker: format_address(&o.maker),
        signer: format_address(&o.signer),
        token_id: bigint_to_string(&o.token_id),
        side: o.side,
        limit_price: format_price(collateral, shares),
        size_shares: bigint_to_string(shares),
        maker_amount: bigint_to_string(&o.maker_amount),
        taker_amount: bigint_to_string(&o.taker_amount),
        signature_type: o.signature_type,
        created_at_ms: bigint_to_u64(&o.timestamp_ms),
        metadata: format_address(&o.metadata),
        builder: format_address(&o.builder),
        is_taker: o.is_taker,
        fill_amount: bigint_to_string(&o.fill_amount),
        fee: bigint_to_string(&o.fee_amount),
        tx: Some(tx).into(),
    }
}
