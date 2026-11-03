#!/usr/bin/env bash
# Checks each package's tests/assertions.yaml against real Polygon blocks.
# Run before each release, after `make build-all` (the manifests point at the
# built wasm) and `substreams auth`. Needs substreams and jq.
#   scripts/run_test_files.sh            # every package
#   scripts/run_test_files.sh exchange   # one package
#
# The files use the `substreams run --test-file` format and are passed to it,
# but as of substreams v1.26 that runner never fails a run and only reports in
# the interactive TUI. So this script also evaluates every assertion itself with
# jq over the `-o jsonl` output, and exits non-zero on any mismatch.
set -euo pipefail

EP=${ENDPOINT:-polygon.streamingfast.io:443}
root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
pkgs=("$@")
if [ ${#pkgs[@]} -eq 0 ]; then
  pkgs=(exchange ctf neg-risk-ctf neg-risk-adapter collateral wallet-factory resolution trader-index trades)
fi

failed=0
for pkg in "${pkgs[@]}"; do
  dir="$root/polymarket-$pkg"
  file="$dir/tests/assertions.yaml"
  # module<TAB>block<TAB>path<TAB>expect, one line per assertion.
  awk '
    /- module:/ { m=$NF }
    /block:/    { b=$NF }
    /path:/     { sub(/^[^:]*: */, ""); gsub(/^'\''|'\''$/, ""); p=$0 }
    /expect:/   { sub(/^[^:]*: */, ""); gsub(/^"|"$/, ""); print m "\t" b "\t" p "\t" $0 }
  ' "$file" > "$tmp/tests.tsv"
  # One run per distinct (module, block), so assertions on far-apart blocks don't
  # stream the whole range between them.
  : > "$tmp/out.jsonl"
  cut -f1,2 "$tmp/tests.tsv" | sort -u > "$tmp/runs.tsv"
  while IFS=$'\t' read -r module blk; do
    echo "== $pkg $module @ $blk"
    (cd "$dir" && substreams run substreams.yaml "$module" -e "$EP" \
      -s "$blk" -t "$((blk + 1))" -o jsonl --test-file tests/assertions.yaml \
      >> "$tmp/out.jsonl" 2> "$tmp/err.log" < /dev/null) || { cat "$tmp/err.log" >&2; exit 1; }
  done < "$tmp/runs.tsv"
  while IFS=$'\t' read -r module block path expect; do
    actual=$(jq -r --arg m "$module" --argjson b "$block" \
      "select(.[\"@block\"] == \$b and .[\"@module\"] == \$m) | .[\"@data\"] | $path" "$tmp/out.jsonl")
    if [ "$actual" == "$expect" ]; then
      echo "   ok   $module $block $path"
    else
      echo "   FAIL $module $block $path: expected '$expect', got '$actual'"
      failed=1
    fi
  done < "$tmp/tests.tsv"
done
[ "$failed" -eq 0 ] || { echo "assertion failures" >&2; exit 1; }
echo "all assertion files passed"
