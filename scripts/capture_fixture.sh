#!/usr/bin/env bash
# Captures one real Polygon block as a shared test fixture:
#   scripts/capture_fixture.sh <block>
#   e.g. scripts/capture_fixture.sh 96600000
# writes testdata/polygon_<block>.binpb.b64 at the repo root (shared by every
# package, since one block usually has events for several), which tests load with
# polymarket_substreams_common::load_block_b64 (feature "fixtures").
#
# Needs `firecore` (github.com/streamingfast/firehose-core releases) and a
# StreamingFast token in FIREHOSE_API_TOKEN, SUBSTREAMS_API_TOKEN or
# FIREHOSE_API_KEY / SUBSTREAMS_API_KEY (`substreams auth` sets them up).
set -euo pipefail

block=${1:?usage: $0 <block>}
endpoint=${ENDPOINT:-polygon.streamingfast.io:443}
root=$(cd "$(dirname "$0")/.." && pwd)
dir="$root/testdata"

export FIREHOSE_API_TOKEN=${FIREHOSE_API_TOKEN:-${SUBSTREAMS_API_TOKEN:-}}
export FIREHOSE_API_KEY=${FIREHOSE_API_KEY:-${SUBSTREAMS_API_KEY:-}}

mkdir -p "$dir"
out="$dir/polygon_$block.binpb.b64"
firecore tools firehose-single-block-client "$endpoint" "$block" \
  -o bytes --bytes-encoding=base64 > "$out.tmp"
mv "$out.tmp" "$out"
echo "wrote $out ($(wc -c < "$out") bytes)"
