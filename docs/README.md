# Bevy Raytraced Audio

This book documents the CPU acoustic core, Bevy 2D and 3D adapters, supported
versions, and example workflows. Start with the
[quick start in the repository README](https://github.com/sagan-software/bevy-raytraced-audio#quick-start).

Listener rays estimate source visibility, wall transmission, room reverb, and
the direction of outdoor ambience. `RaytracedAudioPlayer` applies smoothed
frequency-dependent muffling and reverb through Bevy's audio playback system.
Plain `AudioPlayer` emitters receive scalar volume changes. Optional debug
plugins animate the rays; separate image-source components expose geometric
first-order reflection paths. GPU tracing and mesh extraction remain future
work.

The [example guide](EXAMPLES.md) covers nine native and browser scenes,
including an editable sandbox and focused door, reverb, ambience, and wall
transmission examples. Each generated browser page includes its Rust source.
The gallery and this book share one GitHub Pages site.
