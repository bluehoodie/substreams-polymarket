//! Owned `eth::Block` vs `&eth::BlockLazyView` for the extraction step every
//! handler shares: decode the block, keep the CTF Exchange logs, decode
//! `OrderFilled` / `OrdersMatched` and build each event's transaction context.
//! The owned side is the v1.0.0 handler path; the lazy side is the current one.
//!
//! Run with `cargo bench -p polymarket-substreams-common --bench lazy_vs_owned`.

#[path = "fixture.rs"]
mod fixture;

use buffa::view::LazyMessageView;
use buffa::Message;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use polymarket_substreams_common::abi::ctf_exchange::events::{OrderFilled, OrdersMatched};
use polymarket_substreams_common::{
    block_logs, format_address, tx_context, BlockLog, TxContext, CTF_EXCHANGE,
};
use substreams_ethereum::pb::eth::v2 as eth;

/// What a handler keeps per event: the decoded fields it formats plus its
/// transaction context. Only read through `black_box`.
#[allow(dead_code)]
struct Out {
    who: String,
    ctx: TxContext,
}

fn owned(bytes: &[u8]) -> Vec<Out> {
    let blk = eth::Block::decode_from_slice(bytes).expect("fixture decodes");
    let mut out = Vec::new();
    for log in blk.logs() {
        if log.log.address != CTF_EXCHANGE {
            continue;
        }
        let ctx = || TxContext {
            tx_hash: format_address(&log.receipt.transaction.hash),
            log_index: log.log.block_index as u64,
            block_number: blk.number,
            timestamp: blk.timestamp_seconds(),
        };
        if OrderFilled::match_log(log.log) {
            if let Ok(e) = OrderFilled::decode(log.log) {
                out.push(Out {
                    who: format_address(&e.maker),
                    ctx: ctx(),
                });
            }
        } else if OrdersMatched::match_log(log.log) {
            if let Ok(e) = OrdersMatched::decode(log.log) {
                out.push(Out {
                    who: format_address(&e.taker_order_maker),
                    ctx: ctx(),
                });
            }
        }
    }
    out
}

fn lazy(bytes: &[u8]) -> Vec<Out> {
    let view = eth::BlockLazyView::decode_lazy(bytes).expect("fixture decodes");
    let blk = block_logs(&view, |a| a == CTF_EXCHANGE).expect("fixture decodes");
    let mut out = Vec::new();
    for BlockLog { tx_hash, log } in &blk.logs {
        let ctx = || tx_context(tx_hash, log, blk.number, blk.timestamp);
        if OrderFilled::match_log(log) {
            if let Ok(e) = OrderFilled::decode(log) {
                out.push(Out {
                    who: format_address(&e.maker),
                    ctx: ctx(),
                });
            }
        } else if OrdersMatched::match_log(log) {
            if let Ok(e) = OrdersMatched::decode(log) {
                out.push(Out {
                    who: format_address(&e.taker_order_maker),
                    ctx: ctx(),
                });
            }
        }
    }
    out
}

fn bench(c: &mut Criterion) {
    let bytes = fixture::block().encode_to_vec();
    assert_eq!(owned(&bytes).len(), lazy(&bytes).len());

    let mut group = c.benchmark_group(format!("extract_exchange_events/{}B_block", bytes.len()));
    group.bench_function("owned", |b| b.iter(|| owned(black_box(&bytes))));
    group.bench_function("lazy", |b| b.iter(|| lazy(black_box(&bytes))));
    group.finish();
}

criterion_group!(benches, bench);
criterion_main!(benches);
