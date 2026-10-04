# Bevy template discovery

Research date: 2026-10-04.

## Expected source

Owner-local Cubewright notes identify the Bevy template as:

```text
~/Code/gitlab.com/liamcurry/templates/template-bevy
```

Those notes record template commit `9d5f28a` dated 2026-09-04, a Bevy 0.19.1 networked 2D demo. They describe edition 2024, resolver 3, Rust workspace MSRV 1.96.1, Clippy MSRV 1.96.1, a Nix flake using `rust-overlay`, `crane`, and `treefmt-nix`, and Bevy-specific validation. These are discovery notes only; inspect the exact template source before copying or relying on any setting.

## Access attempts from this host

- No `template-bevy` checkout was found under `/home/sagan/Code` or `/home/sagan/Sync`.
- Authenticated GitLab API lookup and project search did not expose `liamcurry/templates/template-bevy` to the current account.
- GitLab SSH returned a project-not-found or permission response.
- The recorded Tailscale alias for the NixOS machine was offline. A separate online `nixos` peer refused port 22 and the `tailscale ssh` attempt closed without a session.
- The user's previous project notes contain the path and commit summary but not the actual template files.

No credentials or SSH keys were changed. No template files were copied. This repository therefore remains a docs-only plan until the owner restores SSH access, supplies a reachable path, or authorizes the fallback scaffold.

## Required import review

When the template becomes available:

1. Record its exact remote URL, commit, and working-tree state.
2. Inspect `AGENTS.md`, reviewer rules, Cargo manifests, Rust and Clippy MSRV settings, `flake.nix`, `flake.lock`, Nix checks, profiles, lint configuration, CI, examples, benchmarks, and assets.
3. Generate this project from the template or copy its documented scaffold, preserving only applicable Bevy, Nix, and developer conventions.
4. Remove unrelated gameplay, networking, and asset content without copying encrypted secrets or dirty user work.
5. Run the template's baseline checks before making project-specific changes.

The final README and plan must record what was retained and changed. `nix build`, `nix run`, and `nix flake check` remain unverified until this import and first build succeed.
