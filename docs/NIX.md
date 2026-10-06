# Nix workflows

The flake pins the Rust toolchain, native Bevy libraries, formatters, test and
coverage tools, benchmark dependencies, and mdBook.

## Common commands

```sh
nix develop
nix build
nix run
nix flake check
```

`nix build` builds the CPU propagation library. `nix run` opens the 2D tutorial.
`nix flake check` includes formatting, Clippy, Bevy feature checks, tests,
documentation, benchmarks, coverage, and the WebAssembly example compilation.

Run the named native scenes with:

```sh
nix run .#demo-2d
nix run .#demo-3d
nix run .#stress-2d
nix run .#stress-3d
```

Build the browser gallery and serve it locally with:

```sh
nix run .#web-build
nix run .#web-serve
```

The build writes a unique generated site directory under `target/` and copies
the book, each example's WebAssembly output, and its audio asset. The server
listens on `http://127.0.0.1:8000`.

## Build dependencies

The Linux development shell supplies ALSA, udev, X11, Wayland, Vulkan loader,
Clang, LLD, CMake, and `pkg-config`. Cargo handles Rust libraries. The browser
build installs the lockfile-matched `wasm-bindgen-cli` 0.2.129 under ignored
`target/web-tools` output.

No command requires a GPU or an audio output device to build or test the core.
Native examples use Bevy's renderer and the audio device available at runtime.

## Release checks

Before publishing, run `nix build`, `nix run .#check`, and `nix flake check`.
Run the Pages workflow after the repository's GitHub Pages source is set to
GitHub Actions. The workflow builds one static artifact, then deploys it through
GitHub Pages.
