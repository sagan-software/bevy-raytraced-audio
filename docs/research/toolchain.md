# Bevy and Rust version findings

Research date: 2026-10-06.

## Bevy support targets

The official Bevy release list contains stable 0.17.3, 0.18.1, and 0.19.1. It lists `0.20.0-rc.2`, released 2026-09-28, as the latest 0.20 build. Bevy 0.20 support therefore remains a release-candidate check until stable 0.20 is tested. Sources: [Bevy releases](https://github.com/bevyengine/bevy/releases) and the [0.20 milestone](https://github.com/bevyengine/bevy/milestone/43).

The Bevy root manifests declare Rust 1.88.0 for 0.17.3, 1.89.0 for 0.18.1, 1.95.0 for 0.19.1, and 1.96.0 for 0.20.0-rc.2. The project uses Rust 1.96.1 as its MSRV so one toolchain can build every selected engine version.

## Rust stable support

Rust 1.99.0 became stable on 2026-10-01. Rust 1.96, 1.97, 1.98, and 1.99 are the four latest stable minor versions in this research snapshot. The project MSRV and Clippy `msrv` are set to 1.96.1; CI tests 1.96.1, 1.97.1, 1.98.1, and 1.99.0. Rust 1.97.1 is used instead of 1.97.0 because the official release list shows the patch release is available. Source: [Rust release announcements](https://blog.rust-lang.org/releases/).

## Template and lint sources

The imported [`template-bevy`](https://github.com/sagan-software/template-bevy) clone is at `da85b65ffbad26f78d23c953d379a5b0d624e1ed`. It uses Bevy 0.19.1, edition 2024, resolver 3, and Rust 1.96.1. Its Nix flake uses nixpkgs, rust-overlay, crane, and treefmt-nix. See [template research](template.md) for the retained files and import record.

The Sagan Dylints clone is at `0ed03375bd1190a44720a68f189fd9ca5fdf6710`. Its README Quick Start configures `main`, selects `correctness`, `perf`, and `suspicious`, and documents Dylint 6.0.3 with `nightly-2026-07-15`. The project passes those selected groups through `nix run .#dylint`; see [Dylints integration](../DYLINTS.md).
