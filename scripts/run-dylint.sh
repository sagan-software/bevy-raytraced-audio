#!/usr/bin/env bash
set -euo pipefail

project_root="$(git rev-parse --show-toplevel)"
dylints_checkout="${DYLINTS_CHECKOUT:-$(dirname "$project_root")/dylints}"
dylints_checkout="$(cd "$dylints_checkout" && pwd)"

if [[ ! -f "$dylints_checkout/flake.nix" ]]; then
    printf 'Dylints checkout not found: %s\n' "$dylints_checkout" >&2
    printf 'Set DYLINTS_CHECKOUT to its local checkout.\n' >&2
    exit 1
fi

# Keep Bevy's Nix libraries when entering the Dylints toolchain shell.
nix develop "$project_root" --command bash \
    "$project_root/scripts/run-dylint-project-shell.sh" \
    "$project_root" "$dylints_checkout"
