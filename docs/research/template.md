# Bevy template import

## Source

- Repository: [sagan-software/template-bevy](https://github.com/sagan-software/template-bevy)
- Local reference clone: `/home/sagan/Code/github.com/sagan-software/template-bevy`
- Imported commit: `da85b65ffbad26f78d23c953d379a5b0d624e1ed` (`Add generic Bevy project template`)
- Branch at import: `main`; clone was clean and matched `origin/main`.
- License: MIT, retained from the template.
- Import date: 2026-10-04.

The tracked template files were copied into this repository without its `.git` directory. The local project keeps its existing `main` history and research commits. A fetch on 2026-10-05 found the template's `main` branch still at the imported commit and no `master` branch.

## Retained scaffold

The import retains the template's Nix flake and lockfile, Rust toolchain and formatter settings, strict Clippy and Rust workspace lints, Nix build/test/coverage/benchmark workflows, native Linux dependencies, and AI tooling setup. The template's networked game, tests, and benchmarks were removed and replaced with acoustic core tests, Bevy adapters, example workloads, and browser builds.

The root Cargo package and Rust crate references were renamed to `bevy-raytraced-audio` and `bevy_raytraced_audio`. The workspace MSRV and Clippy `msrv` are `1.96.1`; `rust-toolchain.toml` pins `1.99.0`. Local compatibility checks use 1.99.0, and CI is configured for Rust 1.96.1 through 1.99.0.

The root Cargo workspace follows the Sagan Dylints Quick Start on `main`, selecting the `correctness`, `perf`, and `suspicious` groups. The configured Dylint gate passes; see [Dylints integration](../DYLINTS.md).

## Integration result

The checked-in workspace targets Rust 1.96.1. The Nix gates now cover the acoustic crates, Bevy compatibility features, examples, browser builds, tests, coverage, and benchmarks. The configured CI matrix spans four Rust stable releases; local checks ran with Rust 1.99.0.
