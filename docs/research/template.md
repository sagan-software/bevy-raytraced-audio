# Bevy template import

## Source

- Repository: [sagan-software/template-bevy](https://github.com/sagan-software/template-bevy)
- Local reference clone: `$HOME/Code/github.com/sagan-software/template-bevy`
- Imported commit: `da85b65ffbad26f78d23c953d379a5b0d624e1ed` (`Add generic Bevy project template`)
- Updated reference commit: `1f515985e4bd4cf63f0f52f92e3bb5b6353dbe13` (`Include portable WGSL shader materials in every generated game`)
- Reference branch: `main`; fast-forwarded from the imported commit on 2026-10-06.
- License: MIT, retained from the template.
- Import date: 2026-10-04.

The tracked template files were copied into this repository without its `.git` directory. The local project keeps its existing `main` history and research commits. The upstream repository uses `main`; it has no `master` branch.

## Retained scaffold

The import retains the template's Nix flake and lockfile, Rust toolchain and formatter settings, strict Clippy and Rust workspace lints, Nix build/test/coverage/benchmark workflows, native Linux dependencies, and AI tooling setup. The template's networked game, tests, and benchmarks were removed and replaced with acoustic core tests, Bevy adapters, example workloads, and browser builds.

The root Cargo package and Rust crate references were renamed to `bevy-raytraced-audio` and `bevy_raytraced_audio`. The workspace MSRV and Clippy `msrv` are `1.96.1`; `rust-toolchain.toml` pins `1.99.0`. Local compatibility checks use 1.99.0, and CI is configured for Rust 1.96.1 through 1.99.0.

The updated template adds a configurable workspace generator, a tested sccache and worktree artifact helper, and embedded WGSL fragment materials. This project uses the cache helper and its safety tests. Its explicit build lane keeps concurrent worktree outputs separate. The template's fragment materials render mesh surfaces; they do not implement GPU acoustic traversal, so this project does not treat them as its audio GPU backend.

The root Cargo workspace follows the latest Sagan Dylints Quick Start on `main`, selecting the `correctness`, `perf`, and `suspicious` groups. See [Dylints integration](../DYLINTS.md).

## Integration result

The checked-in workspace targets Rust 1.96.1. The Nix gates now cover the acoustic crates, Bevy compatibility features, examples, browser builds, tests, coverage, and benchmarks. The configured CI matrix spans four Rust stable releases; local checks ran with Rust 1.99.0.
