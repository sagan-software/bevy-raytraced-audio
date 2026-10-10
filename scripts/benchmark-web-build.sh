#!/usr/bin/env bash
set -euo pipefail
out=${1:-target/performance/web}
bench_profile=${2:-wasm-release}
bench_target=${CARGO_TARGET_DIR:-target}
version=$(python3 -c 'import tomllib; print(next(p["version"] for p in tomllib.load(open("Cargo.lock", "rb"))["package"] if p["name"] == "wasm-bindgen"))')
if test -x target/web-tools/bin/wasm-bindgen; then
  export PATH="$PWD/target/web-tools/bin:$PATH"
fi
if ! command -v wasm-bindgen >/dev/null || test "$(wasm-bindgen --version)" != "wasm-bindgen $version"; then
  cargo install --locked --version "$version" --root target/web-tools wasm-bindgen-cli
  export PATH="$PWD/target/web-tools/bin:$PATH"
fi
cargo build --locked -p acoustic-performance --lib --target wasm32-unknown-unknown --profile "$bench_profile"
mkdir -p "$out/pkg"
wasm-bindgen --target web --out-dir "$out/pkg" "$bench_target/wasm32-unknown-unknown/$bench_profile/acoustic_performance.wasm"
cp tests/performance/web/index.html tests/performance/web/runner.mjs tests/performance/web/cache-runner.mjs tests/performance/web/compare-workloads.mjs "$out/"
python3 - "$out" "$bench_profile" <<'PYMETA'
import json, pathlib, subprocess, sys
pathlib.Path(sys.argv[1], 'build.json').write_text(json.dumps({
    'profile': sys.argv[2], 'git': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
    'dirty': subprocess.check_output(['git', 'status', '--porcelain'], text=True),
    'rustc': subprocess.check_output(['rustc', '-Vv'], text=True)
}, indent=2)+'\n')
PYMETA
echo "Serve with: python3 -m http.server 8765 --directory $out"
