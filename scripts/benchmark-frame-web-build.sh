#!/usr/bin/env bash
# Build the identical three application workloads for the collaborative real browser.
set -euo pipefail
out=${1:-target/performance/frame-web}
export PATH="$PWD/target/web-tools/bin:$PATH"
cargo build --locked -p bevy-raytraced-audio-examples --target wasm32-unknown-unknown \
  --profile wasm-release --features webgl2 --example showcase --example stress_2d --example stress_3d
mkdir -p "$out"
cp -a website/. "$out/"
node website/generate-pages.mjs "$out"
for name in showcase stress_2d stress_3d; do
  package="$out/examples/$name/pkg"
  mkdir -p "$package"
  wasm-bindgen --target web --out-name app --out-dir "$package" \
    "target/wasm32-unknown-unknown/wasm-release/examples/$name.wasm"
  gzip -n -9 -k -f "$package/app_bg.wasm"
  # Generated artifacts only; source assets stay in their original tracked location.
  ln -sfn "$PWD/examples/raytraced-audio/assets" "$out/examples/$name/assets"
done
