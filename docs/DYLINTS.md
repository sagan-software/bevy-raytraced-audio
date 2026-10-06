# Sagan Dylints

This workspace follows the [Quick Start](https://github.com/sagan-software/dylints#quick-start) in the Sagan Dylints README. It tracks upstream `main` and selects its `correctness`, `perf`, and `suspicious` groups through Cargo workspace metadata. The local reference clone was fast-forwarded to `0ed03375bd1190a44720a68f189fd9ca5fdf6710` on 2026-10-06. The repository uses `main`; it has no `master` branch.

## Cargo setup

The workspace root contains:

```toml
[workspace.metadata.dylint]
libraries = [
    { git = "https://github.com/sagan-software/dylints", branch = "main", pattern = ["lints/correctness", "lints/perf", "lints/suspicious"] },
]
```

This matches the current README Quick Start. It selects lints during development and CI without adding a runtime dependency. The selected libraries are listed in `unexpected_cfgs` so stable Rust builds recognize their conditional attributes.

## Run the lint gate

The project wraps the Dylint Quick Start command with a Nix development shell that provides Bevy's native dependencies:

```sh
rustup toolchain install nightly-2026-07-15 --component rustc-dev --component llvm-tools-preview --component rust-src
nix run .#dylint
```

The Nix wrapper supplies Dylint 6.0.3, its prebuilt driver, and Bevy's native build libraries. It uses the installed Quick Start nightly from `RUSTUP_HOME`, defaulting to `~/.rustup`. This lets Dylint resolve the `rust-toolchain.toml` files in its Git checkouts. The wrapper clears Cargo build-directory and compiler-wrapper variables before invoking Dylint.

The refreshed Quick Start configuration passed `nix run .#dylint` on 2026-10-06. The run built and checked the `correctness`, `perf`, and `suspicious` groups against Bevy 0.17.3, 0.18.1, 0.19.1, and 0.20.0-rc.2. Run `nix run .#check` for strict Clippy, compatibility checks, tests, doctests, docs, benchmark compilation, and coverage.

Upstream lint changes can change compiler and source requirements. Review the Dylints README and rerun the version matrix when updating the branch or runner.
