# Glossary

## Acoustic propagation

The simulation of sound traveling between a source and listener through scene geometry. The simulation estimates direct sound, occlusion, reflections, transmission, and room response. A separate audio processor applies those estimates to samples.

## Acoustic surface

An entity or mesh explicitly included in acoustic scene geometry and assigned acoustic material properties. Render materials do not define acoustic behavior unless an adapter documents that mapping.

## Direct sound

The path from an emitter to the listener without a reflected segment. The path may be attenuated by distance, air, and surfaces.

## Early reflections

Discrete reflected paths with enough arrival-time and direction information to render individually.

## Late reverb

A compact approximation of the dense, later portion of a room response. It does not require tracing every late path as a separate audio voice.

## Emitter

An entity whose position and optional audio stream act as a sound source for the simulation.

## Listener

The position, orientation, and optional ear model used to evaluate propagation and spatial rendering.

## Ray-traced audio

The proposed method uses rays or sampled paths to estimate acoustic propagation. The term describes simulation of path energy and arrival information; it does not mean tracing the waveform sample by sample.

## CPU backend and GPU backend

The CPU backend is the portable reference implementation. The GPU backend runs compatible propagation work in compute shaders and returns acoustic parameters. GPU work is optional and must not be required to build or run the CPU backend.

## 2D mode and 3D mode

2D mode models propagation on a plane using 2D geometry. 3D mode models propagation through 3D geometry. 2D is not a rendering-only projection of the 3D solver.

## Brownfield integration

Adding the plugin to an existing Bevy application while retaining unrelated systems, assets, audio, and plugin configuration.
