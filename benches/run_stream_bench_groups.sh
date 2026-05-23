#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

groups=(
  prod_binary_pipeline
  prod_varints
  prod_signed_varints
)

for group in "${groups[@]}"; do
  printf '\n==> cargo bench --bench stream (%s)\n' "$group"
  QUBIT_IO_STREAM_BENCH_GROUP="$group" cargo bench --bench stream "$@"
done
