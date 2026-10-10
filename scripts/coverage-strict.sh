#!/usr/bin/env bash
# Report the requested production-code line gate without excluding hard-to-test production files.
set -euo pipefail
out=${1:-target/performance/coverage}
mkdir -p "$out"
# report uses default-members unless packages are selected explicitly.
mapfile -t members < <(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; d=json.load(sys.stdin); ids=set(d["workspace_members"]); print("\n".join(p["name"] for p in d["packages"] if p["id"] in ids))')
packages=()
for member in "${members[@]}"; do packages+=(-p "$member"); done
cargo llvm-cov --workspace --locked --profile coverage --no-report
cargo llvm-cov report "${packages[@]}" --profile coverage --json --output-path "$out/coverage.json" \
  --ignore-filename-regex '(^|/)(tests|benches|examples)/'
cargo llvm-cov report "${packages[@]}" --profile coverage --html --output-dir "$out" \
  --ignore-filename-regex '(^|/)(tests|benches|examples)/'
cargo llvm-cov report "${packages[@]}" --profile coverage --show-missing-lines --fail-under-lines 100 \
  --ignore-filename-regex '(^|/)(tests|benches|examples)/'
