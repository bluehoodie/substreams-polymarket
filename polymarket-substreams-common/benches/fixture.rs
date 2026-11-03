//! Synthetic Polygon-shaped block shared by the `lazy_vs_owned` bench and the
//! `lazy_parity` test.
//!
//! The container these were written in cannot reach a Firehose endpoint, so the
//! block is built rather than captured. It is sized after a busy Polygon block:
//! 200 transactions, each with calldata, nested calls carrying storage and
//! balance changes, and ERC20 `Transfer` logs from unrelated tokens. Every 7th
//! transaction also hits the CTF Exchange (two `OrderFilled`, one
//! `OrdersMatched`), and every 37th transaction failed, so its logs must be
//! skipped. Everything is derived from the transaction index, so the block is
//! the same on every run.
#![allow(dead_code)]

use buffa_types::google::protobuf::Timestamp;
use polymarket_substreams_common::CTF_EXCHANGE;
use substreams_ethereum::pb::eth::v2 as eth;

pub const TIMESTAMP: i64 = 1_700_000_000;
pub const NUMBER: u64 = 70_000_000;
pub const TXS: u32 = 200;

const ORDER_FILLED: [u8; 32] =
    hex_literal::hex!("d543adfd945773f1a62f74f0ee55a5e3b9b1a28262980ba90b1a89f2ea84d8ee");
const ORDERS_MATCHED: [u8; 32] =
    hex_literal::hex!("174b3811690657c217184f89418266767c87e4805d09680c39fc9c031c0cab7c");
const ERC20_TRANSFER: [u8; 32] =
    hex_literal::hex!("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef");

fn bytes(seed: u32, tag: u8, len: usize) -> Vec<u8> {
    (0..len)
        .map(|i| (seed as usize * 31 + i * 7 + tag as usize * 13) as u8)
        .collect()
}

fn addr(seed: u32, tag: u8) -> Vec<u8> {
    bytes(seed, tag, 20)
}

fn word_addr(a: &[u8]) -> Vec<u8> {
    let mut w = vec![0u8; 12];
    w.extend_from_slice(a);
    w
}

fn word(v: u64) -> Vec<u8> {
    let mut w = vec![0u8; 24];
    w.extend_from_slice(&v.to_be_bytes());
    w
}

fn big(v: u64) -> eth::BigInt {
    eth::BigInt {
        bytes: v.to_be_bytes().to_vec(),
    }
}

fn log(address: Vec<u8>, topics: Vec<Vec<u8>>, data: Vec<u8>, block_index: u32) -> eth::Log {
    eth::Log {
        address,
        topics,
        data,
        index: block_index,
        block_index,
        ordinal: block_index as u64,
    }
}

fn erc20_transfer(i: u32, k: u8, block_index: u32) -> eth::Log {
    log(
        addr(i, 100 + k),
        vec![
            ERC20_TRANSFER.to_vec(),
            word_addr(&addr(i, 110 + k)),
            word_addr(&addr(i, 120 + k)),
        ],
        word(1_000 + i as u64),
        block_index,
    )
}

fn order_filled(i: u32, k: u8, block_index: u32) -> eth::Log {
    let mut data = Vec::new();
    data.extend(word(k as u64 % 2)); // side
    data.extend(word(0x1234 + i as u64)); // token_id
    data.extend(word(1_000_000 + i as u64)); // maker_amount_filled
    data.extend(word(2_000_000 + i as u64)); // taker_amount_filled
    data.extend(word(500)); // fee
    data.extend(bytes(i, 1, 32)); // builder
    data.extend(bytes(i, 2, 32)); // metadata
    log(
        CTF_EXCHANGE.to_vec(),
        vec![
            ORDER_FILLED.to_vec(),
            bytes(i, 10 + k, 32),
            word_addr(&addr(i, 20 + k)),
            word_addr(&addr(i, 30 + k)),
        ],
        data,
        block_index,
    )
}

fn orders_matched(i: u32, block_index: u32) -> eth::Log {
    let mut data = Vec::new();
    data.extend(word(1)); // side
    data.extend(word(0x1234 + i as u64)); // token_id
    data.extend(word(3_000_000)); // maker_amount_filled
    data.extend(word(4_000_000)); // taker_amount_filled
    log(
        CTF_EXCHANGE.to_vec(),
        vec![
            ORDERS_MATCHED.to_vec(),
            bytes(i, 40, 32),
            word_addr(&addr(i, 41)),
        ],
        data,
        block_index,
    )
}

fn call(i: u32, c: u32) -> eth::Call {
    eth::Call {
        index: c,
        depth: c,
        caller: addr(i, 50 + c as u8),
        address: addr(i, 60 + c as u8),
        input: bytes(i + c, 3, 260),
        return_data: bytes(i + c, 4, 64),
        gas_limit: 300_000,
        gas_consumed: 120_000,
        storage_changes: (0..4)
            .map(|s| eth::StorageChange {
                address: addr(i, 60 + c as u8),
                key: bytes(i + s, 5, 32),
                old_value: bytes(i + s, 6, 32),
                new_value: bytes(i + s, 7, 32),
                ordinal: s as u64,
            })
            .collect(),
        balance_changes: (0..2)
            .map(|b| eth::BalanceChange {
                address: addr(i, 70 + b),
                old_value: big(10_000 + i as u64).into(),
                new_value: big(9_000 + i as u64).into(),
                ordinal: b as u64,
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}

pub fn block() -> eth::Block {
    let mut block_index = 0u32;
    let mut next = || {
        block_index += 1;
        block_index - 1
    };
    let transaction_traces = (0..TXS)
        .map(|i| {
            let mut logs: Vec<eth::Log> = (0..3).map(|k| erc20_transfer(i, k, next())).collect();
            if i % 7 == 0 {
                logs.push(order_filled(i, 0, next()));
                logs.push(order_filled(i, 1, next()));
                logs.push(orders_matched(i, next()));
            }
            let status = if i % 37 == 36 {
                eth::TransactionTraceStatus::Failed
            } else {
                eth::TransactionTraceStatus::Succeeded
            };
            eth::TransactionTrace {
                to: addr(i, 80),
                from: addr(i, 81),
                nonce: i as u64,
                gas_price: big(30_000_000_000).into(),
                gas_limit: 500_000,
                gas_used: 210_000,
                value: big(0).into(),
                input: bytes(i, 8, 600),
                v: vec![1],
                r: bytes(i, 9, 32),
                s: bytes(i, 11, 32),
                index: i,
                hash: bytes(i, 12, 32),
                status: status.into(),
                receipt: eth::TransactionReceipt {
                    cumulative_gas_used: 210_000 * (i as u64 + 1),
                    logs_bloom: bytes(i, 14, 256),
                    logs,
                    ..Default::default()
                }
                .into(),
                calls: (0..3).map(|c| call(i, c)).collect(),
                ..Default::default()
            }
        })
        .collect();

    eth::Block {
        number: NUMBER,
        hash: bytes(0, 15, 32),
        size: 120_000,
        header: eth::BlockHeader {
            number: NUMBER,
            timestamp: Timestamp {
                seconds: TIMESTAMP,
                nanos: 0,
                ..Default::default()
            }
            .into(),
            logs_bloom: bytes(0, 16, 256),
            ..Default::default()
        }
        .into(),
        transaction_traces,
        ..Default::default()
    }
}
