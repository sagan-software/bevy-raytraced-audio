#!/usr/bin/env bash
# Report the requested production-code line gate without excluding hard-to-test production files.
set -euo pipefail
out=${1:-target/performance/coverage}
mkdir -p "$out"
# Older rodio versions require an output stream even for spatial volume controls.
# Isolate those tests on ALSA's null device; never change the user's audio configuration.
if [[ $(uname -s) == Linux ]]; then
  acoustic_alsa_config=$(mktemp)
  trap 'rm -f "$acoustic_alsa_config"' EXIT
  printf 'pcm.!default { type null }\n' > "$acoustic_alsa_config"
  export ALSA_CONFIG_PATH="$acoustic_alsa_config" ACOUSTIC_TEST_AUDIO_DEVICE=1
fi
# report uses default-members unless packages are selected explicitly.
mapfile -t members < <(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; d=json.load(sys.stdin); ids=set(d["workspace_members"]); print("\n".join(p["name"] for p in d["packages"] if p["id"] in ids))')
packages=()
for member in "${members[@]}"; do packages+=(-p "$member"); done
# Discard old feature/version binaries and profiles before collecting a fresh report.
cargo llvm-cov clean --workspace
# llvm-cov clean does not select custom-profile binaries; Cargo must clean these explicitly.
coverage_target=${CARGO_LLVM_COV_TARGET_DIR:-$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"] + "/llvm-cov-target")')}
cargo clean "${packages[@]}" --profile coverage --target-dir "$coverage_target"
features=()
for version in 17 18 19 20; do features+=(--features "bevy-raytraced-audio-compat-0-$version/debug_draw"); done
cargo llvm-cov --workspace --locked --profile coverage "${features[@]}" --no-report
cargo llvm-cov report "${packages[@]}" --profile coverage --json --output-path "$out/coverage.json" \
  --ignore-filename-regex '(^|/)(tests|benches|examples)/'
cargo llvm-cov report "${packages[@]}" --profile coverage --html --output-dir "$out" \
  --ignore-filename-regex '(^|/)(tests|benches|examples)/'
cargo llvm-cov report "${packages[@]}" --profile coverage --lcov --output-path "$out/lcov.info" \
  --ignore-filename-regex '(^|/)(tests|benches|examples)/'
cargo llvm-cov report "${packages[@]}" --profile coverage --show-missing-lines --fail-under-lines 100 \
  --ignore-filename-regex '(^|/)(tests|benches|examples)/'
