# Nix plan

The exact `flake.nix` must come from the owner's Bevy template. Its checkout is currently unavailable; this document specifies expected behavior without claiming that a working flake exists.

## Commands and expected behavior

- `nix build` builds the default workspace package and CPU path without requiring a GPU, window server, or audio output device.
- `nix run` starts a small default demonstration with a native window and procedural audio.
- `nix run .#demo-2d` starts the 2D tutorial.
- `nix run .#demo-3d` starts the 3D tutorial.
- `nix flake check` checks formatting, workspace builds, unit/integration tests, examples, and the portable CPU feature set. GPU device tests remain a separate optional check.
- `nix develop` supplies the template's pinned Rust toolchain, cargo tools, native linkers, shader tools, and project lint commands.

## Dependencies

Preserve the template's Nix inputs and build system unless the audit finds a concrete incompatibility with a workspace library. Keep Rust package dependencies in Cargo files and system tools in the flake. Add only demonstrated system libraries for rodio/CPAL and GPU shader validation. Avoid making a native audio device or GPU device a build-time requirement.

## GPU

The default derivation must compile without GPU hardware. Keep GPU crates/features optional. A machine without a render plugin, compatible adapter, driver, or compute capability must run the CPU path. A separate GPU check may run on a known software or hardware adapter and must record adapter identity.

## Flake acceptance checks

After template import, run `nix flake check`, `nix build`, `nix run -- --help` or the documented demo run, and the named 2D/3D apps from a clean checkout. Confirm `flake.lock` is committed and no command depends on the research host's untracked paths.
