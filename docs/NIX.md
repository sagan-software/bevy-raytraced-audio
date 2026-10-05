# Nix plan

The imported [`template-bevy` flake](research/template.md) is the starting point. It currently builds and runs the template's networked 2D demo; it does not yet build an audio simulation crate or audio demo. Update its package and checks as the reviewed workspace is implemented.

## Commands and expected behavior

- `nix build` currently builds the inherited Bevy 0.19.1 demo. It does not yet build the audio core or prove GPU-free audio operation.
- `nix run` currently starts the template's networked 2D demonstration. After replacing that baseline, it should start a small ray-traced audio demonstration with generated sound.
- Planned after the demo examples exist: `nix run .#demo-2d` starts the 2D tutorial.
- Planned after the demo examples exist: `nix run .#demo-3d` starts the 3D tutorial.
- The imported `nix flake check` checks template formatting, workspace builds, unit/integration tests, template examples, and its current coverage floor. Raise the production-code coverage requirement to at least 90% when the audio workspace is implemented. Keep GPU device tests as a separate optional check.
- `nix develop` supplies the template's pinned Rust toolchain, Cargo tools, native linkers, Bevy lint, formatter, test, coverage, and benchmark commands.

## Dependencies

Preserve the template's Nix inputs and build system unless the audit finds a concrete incompatibility with a workspace library. Keep Rust package dependencies in Cargo files and system tools in the flake. Add only demonstrated system libraries for rodio/CPAL and GPU shader validation. Avoid making a native audio device or GPU device a build-time requirement.

## GPU

The final default derivation must compile without GPU hardware. Keep GPU crates/features optional. A machine without a render plugin, compatible adapter, driver, or compute capability must run the CPU path. A separate GPU check may run on a known software or hardware adapter and must record adapter identity.

## Flake acceptance checks

Run `nix flake check`, `nix build`, and a bounded `nix run` smoke test against the imported baseline now. After the audio examples exist, test the named 2D/3D demo apps from a clean checkout. Confirm `flake.lock` is committed and no command depends on the research host's untracked paths.
