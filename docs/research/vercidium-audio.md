# Vercidium Audio research

Research date: 2026-10-04.

## Organization findings

The [Vercidium Audio GitHub organization](https://github.com/vercidium-audio) listed 18 repositories at the research snapshot. Its organization profile describes the SDK as closed source and the engine integrations as open source. The native Godot and Unreal integration READMEs require a separately downloaded Vercidium SDK and state that commercial use requires a license. The new project should implement its own algorithms and must not include the Vercidium SDK, native libraries, wrapper code, or copied source.

The integration repositories are useful for product concepts and engine-facing component boundaries. Repository licenses vary; inspect each license before reusing any code. This plan uses descriptions and behavior summaries only.

## Complete repository inventory

All entries were shallow-cloned under `/home/sagan/Code/github.com/vercidium-audio/`. The commit column records the checked-out HEAD at clone time.

| Repository | Purpose and relevance | HEAD |
| --- | --- | --- |
| [`vercidium-audio/.github`](https://github.com/vercidium-audio/.github) | Organization profile and shared GitHub configuration | `5529b25`, 2026-10-02 |
| [`godot-mono-openal`](https://github.com/vercidium-audio/godot-mono-openal) | Legacy Godot C# OpenAL plugin, marked deprecated | `155472e`, 2026-08-18 |
| [`support`](https://github.com/vercidium-audio/support) | SDK support and feature-request material | `03cafbf`, 2026-06-30 |
| [`vaudio-dotnet-examples`](https://github.com/vercidium-audio/vaudio-dotnet-examples) | .NET demonstrations of SDK behavior | `ab404f3`, 2026-07-17 |
| [`vaudio-fmod`](https://github.com/vercidium-audio/vaudio-fmod) | FMOD integration | `35df257`, 2026-07-17 |
| [`vaudio-godot-mono-openal-2d`](https://github.com/vercidium-audio/vaudio-godot-mono-openal-2d) | Godot C# 2D integration using OpenAL Soft | `131b2ff`, 2026-10-02 |
| [`vaudio-godot-mono-openal-3d`](https://github.com/vercidium-audio/vaudio-godot-mono-openal-3d) | Godot C# 3D integration using OpenAL Soft | `3466462`, 2026-10-02 |
| [`vaudio-godot-native-openal-2d`](https://github.com/vercidium-audio/vaudio-godot-native-openal-2d) | Native C++ Godot 2D integration using OpenAL Soft | `ec3ddcd`, 2026-10-02 |
| [`vaudio-godot-native-openal-3d`](https://github.com/vercidium-audio/vaudio-godot-native-openal-3d) | Native C++ Godot 3D integration using OpenAL Soft | `50644f5`, 2026-10-02 |
| [`vaudio-godot-native-openal-3d-release`](https://github.com/vercidium-audio/vaudio-godot-native-openal-3d-release) | Godot 3D plugin release binaries; repository now marked deprecated | `6142fb6`, 2026-10-02 |
| [`vaudio-native-wrapper`](https://github.com/vercidium-audio/vaudio-native-wrapper) | General .NET Standard wrapper around the native SDK | `d486102`, 2026-10-02 |
| [`vaudio-native-wrapper-2d`](https://github.com/vercidium-audio/vaudio-native-wrapper-2d) | .NET Standard 2D wrapper | `396f7f6`, 2026-09-21 |
| [`vaudio-native-wrapper-3d`](https://github.com/vercidium-audio/vaudio-native-wrapper-3d) | .NET Standard 3D wrapper | `e4522df`, 2026-09-21 |
| [`vaudio-native-wrapper-common`](https://github.com/vercidium-audio/vaudio-native-wrapper-common) | Shared code for the 2D and 3D .NET wrappers | `9b774d8`, 2026-09-21 |
| [`vaudio-openal`](https://github.com/vercidium-audio/vaudio-openal) | OpenAL Soft integration | `4b63415`, 2026-07-17 |
| [`vaudio-swapper`](https://github.com/vercidium-audio/vaudio-swapper) | .NET wrapper that selects managed or native SDK paths | `2eef782`, 2026-10-02 |
| [`vaudio-unreal`](https://github.com/vercidium-audio/vaudio-unreal) | Experimental Unreal Engine C++ plugin | `2a68ea5`, 2026-10-02 |
| [`vaudio-wwise`](https://github.com/vercidium-audio/vaudio-wwise) | Wwise integration | `b8fdd30`, 2026-07-17 |

## Product concepts worth evaluating

The native Godot 2D and 3D plugin READMEs list real-time muffling, reverb, ambience, visualization, event-based ray tracing, energy and material models, and moving-scene updates. The Unreal integration separates world, surface material, listener, source, continuous trace target, relative source, ambience source, and material asset roles. These concepts map cleanly to explicit Bevy components and resources, but do not prescribe the Rust data model or algorithm.

The source plugins require OpenAL Soft or FMOD. Their documentation describes a separate audio backend for spatialization, filters, and reverb. That is consistent with the Bevy research finding that Bevy's built-in sink API lacks general live effect insertion.

## Clean-room boundary

Use the public descriptions to understand desired behavior. Design the propagation model from published acoustics and ray-tracing references, write new Rust code, and use assets with verified licenses. Do not download or bundle the Vercidium SDK as a build dependency. Do not translate source code from any integration into this project.
