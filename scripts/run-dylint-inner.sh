#!/usr/bin/env bash
set -euo pipefail

cd "$PROJECT_ROOT"
unset CARGO_BUILD_BUILD_DIR OPENSSL_DIR

host_target="$(rustc -vV | awk '/^host:/ { print $2 }')"
nix_toolchain="sagan-lints-dev-2026-07-15-$host_target"
dylint_toolchain="nightly-2026-07-15-$host_target"
toolchain_root="$(rustup toolchain list -v | awk -v name="$nix_toolchain" '$1 == name { print $2; exit }')"

if [[ -z "$toolchain_root" ]]; then
    printf 'Dylints Nix toolchain is not linked in RUSTUP_HOME.\n' >&2
    exit 1
fi

if ! rustup toolchain list | grep -Fq "$dylint_toolchain"; then
    rustup toolchain link "$dylint_toolchain" "$toolchain_root"
fi

# Match the prebuilt Dylint driver to the upstream library toolchain name.
driver_dir="$PROJECT_ROOT/target/dylint-driver/$dylint_toolchain"
mkdir -p "$driver_dir"
ln -sfn "$DYLINT_DRIVER_PATH/$nix_toolchain/dylint-driver" "$driver_dir/dylint-driver"
export RUSTUP_TOOLCHAIN="$dylint_toolchain"
export DYLINT_DRIVER_PATH="$PROJECT_ROOT/target/dylint-driver"
export RUSTFLAGS="-D warnings"

cargo dylint --all --workspace -- --all-targets
