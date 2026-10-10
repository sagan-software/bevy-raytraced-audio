#!/usr/bin/env bash
set -euo pipefail
out=${1:-target/performance/web}
version=$(python3 -c 'import tomllib; print(next(p["version"] for p in tomllib.load(open("Cargo.lock", "rb"))["package"] if p["name"] == "wasm-bindgen"))')
if test -x target/web-tools/bin/wasm-bindgen; then
  export PATH="$PWD/target/web-tools/bin:$PATH"
fi
if ! command -v wasm-bindgen >/dev/null || test "$(wasm-bindgen --version)" != "wasm-bindgen $version"; then
  cargo install --locked --version "$version" --root target/web-tools wasm-bindgen-cli
  export PATH="$PWD/target/web-tools/bin:$PATH"
fi
cargo build --locked -p acoustic-performance --lib --target wasm32-unknown-unknown --profile wasm-release
mkdir -p "$out/pkg"
wasm-bindgen --target web --out-dir "$out/pkg" target/wasm32-unknown-unknown/wasm-release/acoustic_performance.wasm
cp tests/performance/web/index.html tests/performance/web/runner.mjs "$out/"
echo "Serve with: python3 -m http.server 8765 --directory $out"
