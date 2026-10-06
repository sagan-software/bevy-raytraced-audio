#!/usr/bin/env bash
set -euo pipefail

cd "$PROJECT_ROOT"
unset CARGO_BUILD_BUILD_DIR RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER OPENSSL_DIR

if [[ -z "${RUSTUP_HOME:-}" || -z "${DYLINT_DRIVER_PATH:-}" ]]; then
    printf 'Enter the Dylints Nix development shell before running this script.\n' >&2
    exit 1
fi

# Use the host-qualified toolchain name registered by the Dylints Nix flake.
host_target="$(rustc -vV | awk '/^host:/ { print $2 }')"
dylint_toolchain="nightly-2026-07-15-$host_target"
toolchain_root="$RUSTUP_HOME/toolchains/$dylint_toolchain"
if [[ ! -x "$toolchain_root/bin/rustc" || -L "$toolchain_root" ]]; then
    printf 'Install the Dylint Quick Start toolchain in RUSTUP_HOME before running this gate.\n' >&2
    printf '%s\n' \
        "rustup toolchain install nightly-2026-07-15 \\" \
        "  --component rustc-dev --component llvm-tools-preview \\" \
        '  --component rust-src' >&2
    exit 1
fi

# The current Nix package and rustup install share the same full toolchain name.
export RUSTUP_TOOLCHAIN="$dylint_toolchain"
export RUSTFLAGS="-D warnings"

cargo dylint --all --workspace -- --all-targets
