# Bevy template import

## Source

- Repository: [sagan-software/template-bevy](https://github.com/sagan-software/template-bevy)
- Local reference clone: `/home/sagan/Code/github.com/sagan-software/template-bevy`
- Imported commit: `da85b65ffbad26f78d23c953d379a5b0d624e1ed` (`Add generic Bevy project template`)
- Branch at import: `main`; clone was clean and matched `origin/main`.
- License: MIT, retained from the template.
- Import date: 2026-10-04.

The tracked template files were copied into this repository without its `.git` directory. The local project keeps its existing `main` history and research commits.

## Retained scaffold

The import retains the template's Nix flake and lockfile, Rust toolchain and formatter settings, strict Clippy and Rust workspace lints, Nix build/test/coverage/benchmark workflows, native Linux dependencies, and AI tooling setup. The template's source app, tests, and benchmarks remain as a temporary Bevy 0.19.1 networked 2D smoke-test baseline. They are not the ray-traced audio implementation and must be replaced or reshaped after plan review.

The root Cargo package and Rust crate references were renamed to `bevy-raytraced-audio` and `bevy_raytraced_audio`. The workspace MSRV and Clippy `msrv` remain `1.96.1`; the checked-in `rust-toolchain.toml` remains at `1.98.1`, matching the imported baseline. The supported 1.96–1.99 Rust matrix and Bevy 0.17–0.20 matrix are research targets, not verified by this single-version scaffold.

Sagan Dylints library selection was added to root Cargo workspace metadata at pinned revision `483b64d83e38352994d509eacf4a56db1892f1a3`. The existing Dylints checkout was not changed.

## Next import gate

Before audio implementation, inspect the imported manifest, Flake, and tests against the approved crate layout. Remove template-only networking, physics, particle, editor, and MCP dependencies when no audio example or development workflow needs them. Preserve only measured project conventions and infrastructure. Then run the template Nix gates and record separate results for baseline scaffold validation and the requested Bevy/Rust compatibility matrix.
