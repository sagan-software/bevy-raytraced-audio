#!/usr/bin/env bash
# Run under nix develop .#performance. Never benchmark concurrently with builds/tests.
set -euo pipefail
baseline=${1:?Usage: benchmark-native.sh BASELINE [criterion arguments...]}
shift
case "$baseline" in *[!a-zA-Z0-9_-]*|'') echo 'Invalid baseline name' >&2; exit 2;; esac
result_dir="target/performance/$baseline"
if test -e "$result_dir/complete"; then
  echo "Refusing to overwrite completed baseline $baseline" >&2
  exit 2
fi
mkdir -p "$result_dir"
{
  git rev-parse HEAD
  git diff --stat
  rustc -Vv
  uname -a
  printenv RUSTFLAGS CARGO_ENCODED_RUSTFLAGS BEVY_TASK_THREADS || true
  if command -v lscpu >/dev/null; then lscpu; fi
} > "$result_dir/environment.txt"
# Compile every executable before measuring the first one.
cargo bench --locked -p bevy-raytraced-audio -p acoustic-performance -p bevy-raytraced-audio-public-tests --no-run --message-format=json > "$result_dir/build.jsonl"
python3 - "$result_dir" <<'PY'
import json, pathlib, shutil, sys
root = pathlib.Path(sys.argv[1])
(root / 'bin').mkdir(exist_ok=True)
names = {'propagation', 'workloads', 'adapter_schedule'}
found = set()
for line in (root / 'build.jsonl').read_text().splitlines():
    row = json.loads(line)
    target = row.get('target', {})
    if target.get('name') in names and row.get('executable'):
        name = target['name']
        shutil.copy2(row['executable'], root / 'bin' / name)
        found.add(name)
if found != names:
    raise SystemExit(f'Missing executables: {names - found}')
PY
for entry in 'bevy-raytraced-audio propagation' 'acoustic-performance workloads' 'bevy-raytraced-audio-public-tests adapter_schedule'; do
  read -r package bench <<< "$entry"
  "$result_dir/bin/$bench" --bench \
    --save-baseline "$baseline" --sample-size 30 --warm-up-time 1 --measurement-time 3 --noplot "$@" \
    2>&1 | tee "$result_dir/$bench.log"
done
touch "$result_dir/complete"
