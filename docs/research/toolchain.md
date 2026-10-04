# Bevy and Rust version findings

Research date: 2026-10-04.

## Bevy support targets

Bevy's GitHub release page listed stable 0.17.3, 0.18.1, and 0.19.1. Bevy 0.20.0-rc.2 was the current release candidate, published 2026-09-28. The 0.20 support row must remain provisional until a stable release is tested.

Bevy root manifests declare `rust-version` 1.88.0 for 0.17.3, 1.89.0 for 0.18.1, 1.95.0 for 0.19.1, and 1.96.0 for 0.20.0-rc.2. The 0.20 release candidate sets the minimum for a project that supports every requested engine version.

## Rust stable support

Rust 1.99.0 was announced on 2026-10-01. The four latest stable minor versions are 1.96, 1.97, 1.98, and 1.99. Set `rust-version` and Clippy's `msrv` to 1.96.0; do not promise Rust 1.95 while supporting the Bevy 0.20 release candidate.

## Template findings from the owner's local project notes

The Bevy template notes identify `~/Code/gitlab.com/liamcurry/templates/template-bevy`, last observed at commit `9d5f28a` dated 2026-09-04. They describe a Bevy 0.19.1 networked 2D demo, edition 2024, resolver 3, workspace MSRV 1.96.1, and Clippy MSRV 1.96.1. They also record a Nix flake using nixpkgs, rust-overlay, crane, and treefmt-nix with Bevy-oriented checks. These are owner-local notes, not a substitute for inspecting the template checkout.

The template was not available on the current T490 filesystem. Its import status and failed access paths are recorded in [template research](template.md). Do not claim that this project inherits or currently validates the template's flake.

## Dylints findings

The local Sagan Dylints checkout at `~/Code/github.com/sagan-software/dylints` is at HEAD `a36bee004e66257eaa8109e848ad56a8870fe131` and has pre-existing working-tree changes. Its README documents revision `483b64d83e38352994d509eacf4a56db1892f1a3` as the tested setup pin. Use that committed revision as the reproducible project dependency unless the owner chooses a later reviewed commit. Leave the dirty checkout unchanged. The README and AGENTS guide configure Dylint libraries with `[[workspace.metadata.dylint.libraries]]`, a pinned `git` revision, and a `pattern`; this is tool configuration, not an application dependency. It contains a `lints/crates/bevy` library.

The current Sagan runner uses `nightly-2026-07-15` and Dylint 6.0.3. The setup must be tested against this project's actual feature matrix because a Dylint compiler can impose a narrower compiler/dependency range than stable builds.
