#!/usr/bin/env bash
# Report the requested production-code line gate without excluding hard-to-test production files.
set -euo pipefail
out=${1:-target/performance/coverage}
mkdir -p "$out"
cargo llvm-cov --workspace --locked --profile coverage --no-report
cargo llvm-cov report --profile coverage --json --output-path "$out/coverage.json" \
  --ignore-filename-regex '(^|/)(tests|benches|examples)/'
cargo llvm-cov report --profile coverage --html --output-dir "$out" \
  --ignore-filename-regex '(^|/)(tests|benches|examples)/'
cargo llvm-cov report --profile coverage --show-missing-lines --fail-under-lines 100 \
  --ignore-filename-regex '(^|/)(tests|benches|examples)/'
