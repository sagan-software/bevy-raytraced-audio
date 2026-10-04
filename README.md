# Bevy Raytraced Audio

Working name for an independent Rust acoustic propagation and audio-effects project for Bevy, intended for the Sagan Software GitHub group. Independent implementation is the current recommendation; the owner should review whether to audit or reuse the closely related Apache-2.0 `omg-audio` core before implementation. This repository contains research and a proposed plan; it has no implementation yet.

The design goal is an opt-in Bevy plugin that works with existing projects, has separate 2D and 3D packages, uses CPU simulation everywhere, and can use a GPU when available. Full ray-traced filtering and reverb may require an explicit processed-audio source or audio-graph adapter. Bevy's current public audio API does not expose a general effect-insertion point for an already-playing `AudioPlayer<AudioSource>`.

## Review documents

- [Delivery plan](docs/PLAN.md)
- [Proposed architecture and API](docs/DESIGN.md)
- [Version support](docs/COMPATIBILITY.md)
- [Example catalogue](docs/EXAMPLES.md)
- [Test and benchmark plan](docs/TESTING-AND-BENCHMARKS.md)
- [Nix plan](docs/NIX.md)
- [Dylints integration](docs/DYLINTS.md)
- [Asset policy](docs/ASSETS.md)
- [Research index](docs/research/README.md)
- [Glossary](GLOSSARY.md)
- [Template import status](docs/research/template.md)
- [Local cloned repositories](docs/research/local-clones.md)
- [Proposed ADR 0001](docs/adr/0001-crates-and-runtime-boundaries.md)

Research snapshot: 2026-10-04. Repository and release state can change; refresh version and source checks before implementation and release.
