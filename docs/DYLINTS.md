# Dylints integration plan

The local Sagan Dylints clone is at `/home/sagan/Code/github.com/sagan-software/dylints`, HEAD `a36bee004e66257eaa8109e848ad56a8870fe131`, with pre-existing working-tree changes. Its README documents `483b64d83e38352994d509eacf4a56db1892f1a3` as the tested setup pin. Keep this plan pinned to that committed revision unless the owner selects a later reviewed revision. The local checkout was not modified. Its [setup guide](https://github.com/sagan-software/dylints/blob/483b64d83e38352994d509eacf4a56db1892f1a3/docs/usage.md) uses Dylint library metadata in the target workspace. The project should not list lint libraries as application dependencies.

## Proposed workspace configuration

```toml
[[workspace.metadata.dylint.libraries]]
git = "https://github.com/sagan-software/dylints"
rev = "483b64d83e38352994d509eacf4a56db1892f1a3"
pattern = [
  "lints/correctness",
  "lints/perf",
  "lints/suspicious",
  "lints/crates/bevy",
]
```

Keep the pin in the workspace root or root `dylint.toml`, not both. Preserve any template entries and add the Dylints library names to the appropriate unexpected-configuration allowance if the setup guide requires them.

## Verification

Use the template's flake integration and run the pinned Sagan runner against the project. Before claiming the setup works, verify one known diagnostic and one clean example. Keep the Dylint compiler gate separate from the four stable Rust compatibility matrix; the documented Dylints pin uses `nightly-2026-07-15` and Dylint 6.0.3. Review the local runner checkout's existing dirty state before using it; do not clean or overwrite those changes.

The Dylints repository already has a Bevy-specific lint group at `lints/crates/bevy`. Use that group first. Add a new audio/Bevy lint to Dylints only after a repeated project rule has a clear, type-aware detector and positive and negative UI cases. The current research does not justify inventing a new lint.

The exact local runner used in the owner's other project is:

```sh
nix run /home/sagan/Code/github.com/sagan-software/dylints#sagan-lints -- --repo .
```
