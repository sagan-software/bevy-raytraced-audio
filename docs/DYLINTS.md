# Sagan Dylints

This workspace follows the [Quick Start](https://github.com/sagan-software/dylints#quick-start) in the Sagan Dylints README. It tracks upstream `main` and selects its `correctness`, `perf`, and `suspicious` groups through Cargo workspace metadata. The local checkout was updated to `9bc21efeccdd1236e3e64cf7bb607823c60d4600`; upstream has no `master` branch.

## Cargo setup

The workspace root contains:

```toml
[[workspace.metadata.dylint.libraries]]
git = "https://github.com/sagan-software/dylints"
branch = "main"
pattern = ["lints/correctness", "lints/perf", "lints/suspicious"]
```

This config selects lints at development and CI time. It does not add a runtime dependency. The selected libraries are listed in `unexpected_cfgs` so stable Rust builds recognize their conditional attributes.

## Verified command

The project wraps the Dylint Quick Start command with a Nix development shell that provides Bevy's native dependencies:

```sh
nix run .#dylint
```

On 2026-10-06, this command passed all three configured groups across the workspace. It used Dylint 6.0.3 and `nightly-2026-07-15`. Run `nix run .#check` for strict Clippy, Bevy compatibility checks, tests, doctests, docs, benchmark compilation, and coverage.

The Sagan `nix run .#dylint` gate passed on 2026-10-06. The Liamc personal
Rust lint passed when scoped to the changed core package with `--no-deps`.
The full personal-lint dispatcher did not pass: it reported repository-wide
documentation diagnostics and its Bevy 0.18 phase could not find ALSA through
that runner's `PKG_CONFIG_PATH`. Its Markdown phase reported four paragraph
length findings in unchanged documents.

Upstream lint changes can change compiler and source requirements. Review the Dylints README and rerun the version matrix when updating the branch or runner.
