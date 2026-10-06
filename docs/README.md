# Bevy Raytraced Audio

This book documents the CPU propagation core, separate Bevy 2D and 3D adapters,
supported versions, and example workflows. Start with the
[quick start in the repository README](../README.md).

The adapters report per-band direct transmission and first-order reflection
paths. Attach the optional 2D or 3D reflection-path component to inspect each
emitter's valid paths. The adapter scales the existing Bevy audio sink by the
mean direct amplitude gain. Reflection data does not add sample filters,
reflection playback, late reverb, or GPU execution.

The [browser examples](EXAMPLES.md) describe four WebAssembly builds. The static
gallery and this book share one GitHub Pages site.
