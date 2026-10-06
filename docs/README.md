# Bevy Raytraced Audio

This book documents the CPU propagation core, separate Bevy 2D and 3D adapters,
supported versions, and example workflows. Start with the
[quick start in the repository README](../README.md).

The current adapter reports direct paths and first-order reflection paths. It
uses direct occlusion to scale the existing Bevy audio sink. It does not add
sample filters, reflection playback, late reverb, or GPU execution.

The [browser examples](EXAMPLES.md) describe four WebAssembly builds. The static
gallery and this book share one GitHub Pages site.
