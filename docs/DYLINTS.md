# Dylints integration plan

The root `Cargo.toml` now selects Sagan Dylints libraries through `workspace.metadata.dylint.libraries`, pinned to revision `483b64d83e38352994d509eacf4a56db1892f1a3`. This is Dylint's workspace metadata mechanism; lint libraries are not application dependencies. The configured groups are correctness, performance, suspicious, and Bevy-specific.

The local Sagan Dylints clone is at `/home/sagan/Code/github.com/sagan-software/dylints`, HEAD `a36bee004e66257eaa8109e848ad56a8870fe131`, with pre-existing working-tree changes. Its README documents the selected commit as a tested setup pin. The local checkout was not modified. Its [setup guide](https://github.com/sagan-software/dylints/blob/483b64d83e38352994d509eacf4a56db1892f1a3/docs/usage.md) describes workspace metadata and runner usage.

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

Run the pinned Sagan runner against the project. Before claiming the setup works, verify one known diagnostic and one clean example. Keep the Dylint compiler gate separate from the four stable Rust compatibility matrix; the documented Dylints pin uses `nightly-2026-07-15` and Dylint 6.0.3. Do not use or modify the dirty local runner checkout for validation.

The fast runner completed on 2026-10-05 after receiving the native-library paths from this flake, but the imported template baseline did not pass. Clippy reported six errors, including an unused `log` dependency and two deprecated `Atomic::fetch_update` calls under the pinned nightly. The workspace check reported dependency-order issues and 269 errors across the temporary networked demo. No ray-traced audio crate exists yet, so treat this as a baseline failure and rerun after removing the demo scaffold. The custom lint setup and known-diagnostic/clean-example pair are not yet verified as a passing gate.

The Dylints repository already has a Bevy-specific lint group at `lints/crates/bevy`. Use that group first. Add a new audio/Bevy lint to Dylints only after a repeated project rule has a clear, type-aware detector and positive and negative UI cases. The current research does not justify inventing a new lint.

Run the pinned runner from the project root through the Nix development shell. The shell supplies the native libraries needed by Bevy's audio and windowing dependencies:

```sh
nix develop --command nix run github:sagan-software/dylints/483b64d83e38352994d509eacf4a56db1892f1a3 -- --repo . --fast
```
