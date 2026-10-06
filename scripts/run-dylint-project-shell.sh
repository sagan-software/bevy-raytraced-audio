#!/usr/bin/env bash
set -euo pipefail

project_root="${1:?project root is required}"
dylints_checkout="${2:?Dylints checkout is required}"
project_pkg_config_path="${PKG_CONFIG_PATH:-}"
project_ld_library_path="${LD_LIBRARY_PATH:-}"

nix develop "$dylints_checkout#default" \
    --ignore-env \
    --keep-env-var HOME \
    --set-env-var PRIVATE_LINTS_ROOT "$dylints_checkout" \
    --set-env-var PROJECT_ROOT "$project_root" \
    --set-env-var PKG_CONFIG_PATH "$project_pkg_config_path" \
    --set-env-var LD_LIBRARY_PATH "$project_ld_library_path" \
    --command bash "$project_root/scripts/run-dylint-inner.sh"
