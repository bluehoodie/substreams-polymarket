#!/usr/bin/env bash
# Captures one real Polygon block as a test fixture for a package:
#   scripts/capture_fixture.sh <package> <block>
#   e.g. scripts/capture_fixture.sh exchange 84902400
# writes polymarket-<package>/src/testdata/polygon_<block>.binpb.b64, which tests
# load with polymarket_substreams_common::load_block_b64 (feature "fixtures").
#
# Needs `firecore` (github.com/streamingfast/firehose-core releases) and a
# StreamingFast token in FIREHOSE_API_TOKEN, SUBSTREAMS_API_TOKEN or
# FIREHOSE_API_KEY / SUBSTREAMS_API_KEY (`substreams auth` sets them up).
set -euo pipefail

pkg=${1:?usage: $0 <package> <block>}
block=${2:?usage: $0 <package> <block>}
endpoint=${ENDPOINT:-polygon.streamingfast.io:443}
root=$(cd "$(dirname "$0")/.." && pwd)
dir="$root/polymarket-$pkg/src/testdata"
[ -d "$root/polymarket-$pkg" ] || { echo "no package polymarket-$pkg" >&2; exit 1; }

export FIREHOSE_API_TOKEN=${FIREHOSE_API_TOKEN:-${SUBSTREAMS_API_TOKEN:-}}
export FIREHOSE_API_KEY=${FIREHOSE_API_KEY:-${SUBSTREAMS_API_KEY:-}}

mkdir -p "$dir"
out="$dir/polygon_$block.binpb.b64"
firecore tools firehose-single-block-client "$endpoint" "$block" \
  -o bytes --bytes-encoding=base64 > "$out.tmp"
mv "$out.tmp" "$out"
echo "wrote $out ($(wc -c < "$out") bytes)"
