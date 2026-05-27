#!/usr/bin/env bash
#
# Publish every Substreams package in this project to the substreams.dev registry.
#
# Auth: the first `substreams publish` opens an interactive copy/paste form for a
# registry token (from https://substreams.dev/me). Paste your token there once —
# it is cached for the remaining packages. To authenticate up front instead, run
# `substreams auth` before this script. The token is NOT stored in this file.
#
# Usage:
#   ./publish-all.sh                 # publish all packages
#   ./publish-all.sh pkg1 pkg2 ...   # publish only the named package dirs
#
set -euo pipefail

cd "$(dirname "$0")"

ALL_PACKAGES=(
  polymarket-collateral
  polymarket-ctf
  polymarket-exchange
  polymarket-neg-risk-adapter
  polymarket-neg-risk-ctf
  polymarket-wallet-factory
  polymarket-uma-oracle
)

# Use packages passed as arguments, or all of them.
PACKAGES=("$@")
if [ "${#PACKAGES[@]}" -eq 0 ]; then
  PACKAGES=("${ALL_PACKAGES[@]}")
fi

for pkg in "${PACKAGES[@]}"; do
  if [ ! -f "$pkg/substreams.yaml" ]; then
    echo "ERROR: $pkg/substreams.yaml not found — skipping" >&2
    exit 1
  fi
  echo
  echo "=== Publishing ${pkg} ==="
  # --yes auto-confirms the publish prompt. Remove it to review each one.
  ( cd "$pkg" && substreams publish --yes )
  echo "=== Done: ${pkg} ==="
done

echo
echo "Published ${#PACKAGES[@]} package(s)."
