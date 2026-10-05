# Bevy Raytraced Audio

A proposed Rust acoustic propagation and audio-effects project for Bevy. The local repository contains the research plan and the imported `template-bevy` scaffold. It does not yet implement audio propagation.

The bootstrap app remains the template's Bevy 0.19.1 networked 2D demonstration until the plan is reviewed. `nix run` launches that demonstration; it does not demonstrate ray-traced audio. The starter's networking and gameplay code is temporary scaffolding and is not part of the audio design.

## Review documents

- [Delivery plan](docs/PLAN.md)
- [Proposed architecture and API](docs/DESIGN.md)
- [Version support plan](docs/COMPATIBILITY.md)
- [Example catalogue](docs/EXAMPLES.md)
- [Test and benchmark plan](docs/TESTING-AND-BENCHMARKS.md)
- [Nix setup](docs/NIX.md)
- [Dylints integration](docs/DYLINTS.md)
- [Asset policy](docs/ASSETS.md)
- [Research index](docs/research/README.md)
- [Glossary](GLOSSARY.md)
- [Template import record](docs/research/template.md)
- [Proposed ADR 0001](docs/adr/0001-crates-and-runtime-boundaries.md)

## Bootstrap commands

The imported template supplies these commands while the audio crates are being designed:

```sh
nix run
nix build
nix flake check
nix run .#check
```

The current Flake checks validate the template scaffold. The Bevy 0.17–0.20 compatibility matrix, 90% production-code coverage target, audio examples, CPU/GPU parity, and audio benchmarks remain planned gates.

## Research snapshot

Research date: 2026-10-04. The Bevy starter template was copied from [`sagan-software/template-bevy`](https://github.com/sagan-software/template-bevy) at commit `da85b65ffbad26f78d23c953d379a5b0d624e1ed`. See the [template import record](docs/research/template.md) for retained files and the bootstrap boundary.
