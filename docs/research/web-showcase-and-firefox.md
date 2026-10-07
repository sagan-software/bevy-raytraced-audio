# Web showcase and Firefox investigation

Research date: 2026-10-06.

## Bevy example-site pattern

The [official Bevy examples site](https://bevy.org/examples/#audio) groups
examples under category headings and provides direct browser-run links with
source descriptions. Its audio section includes playback, custom decoding,
event playback, state-based music, and 2D and 3D spatial-audio examples. The
site also uses a search field and image-led example cards. The project gallery
uses a category index, search field, and example cards grouped under 2D, 3D,
and stress tests. It features the forest demo above the categories. Each Bevy
build has its own route.

## Vercidium visual review

The public [interactive-demo post](https://www.patreon.com/vercidium/posts/raytraced-audio-155391088)
shows a forest scene, keyboard and mouse controls, performance readouts, an
audio-visualization option, and ray and bounce settings. The public video
[A First Look At Raytraced Audio](https://www.youtube.com/watch?v=u6EuAUjq92k)
shows forest and room scenes, visible sound paths, debugging views, and a
permeation visualization. The video [I Built Raytraced Audio for Godot](https://www.youtube.com/watch?v=A6bPUXTlic8)
shows a separate Godot editor workflow, low-poly world geometry, acoustic
material controls, and propagation settings.

Both videos were downloaded with `yt-dlp` and sampled with FFmpeg into 4×4
contact sheets. The Patreon screenshots were also reviewed. The contact sheets
and screenshots stayed in `/tmp/vercidium-audio-reference`; source images are
not included in the repository. The new forest scene uses procedural Bevy
geometry and its own UI.

The 4×4 sheet for the first video shows a voxel forest, indoor rooms, animated
propagation paths, and debug views. The Godot video shows its editor, per-surface
material fields, and acoustic debug views. Patreon screenshots show forest
controls, performance timings, and audio-visualization and ray-count settings.
These references inform movement, material labels, and diagnostics; this
project does not add unsupported ray-count, bounce-count, or reflection-audio
controls.

The README forest GIF and catalogue image were recorded from the project's
WebAssembly scene. The GIF shows the direct response changing as the listener
moves across the brush screen. The project does not reuse the video or Patreon
images.

## Feature boundary

The reference screenshots show controls for features this project does not
currently implement. The Bevy demo exposes movement, a path-visibility toggle,
and frame diagnostics. It does not expose ray-count, bounce-count, reverb, or
permeation controls. The project computes direct transmission and first-order
reflection geometry. The Bevy adapter applies direct transmission to sink
volume; reflection lines remain visual data without reflection or reverb DSP.

## Firefox reproduction

The published `minimal-3d` route was opened in Firefox 157 headless through
WebDriver. Firefox returned `null` from `canvas.getContext("webgl2")`, even
when the profile enabled `webgl.force-enabled` and `webgl.enable-webgl2`. The
browser log said `AllowWebgl2:false restricts context creation on this system.`
The old page nevertheless changed its status to `Scene ready` and enabled audio
while the Bevy canvas remained at its HTML default size of 300 × 150 pixels.
Bevy also logged a WebGL context failure and a WebAssembly `unreachable`
runtime error. This reproduces the false-ready state in the local headless
environment; it does not establish the WebGL capability of the user's normal
Firefox window.

After the old page's audio button was clicked, its `AudioContext` changed from
`suspended` to `running`. This verifies the browser state transition, not
physical sound from speakers. Physical audio output was not tested.

The shared web loader now probes WebGL2 before loading Bevy and waits for Bevy
to resize the canvas before it reports the scene as ready. If the browser
cannot create a context, the route explains how to enable hardware acceleration
and reload.

The forest initially stopped updating on Bevy's overlapping mutable queries.
Disjoint camera/listener and readout filters fixed those startup conflicts.
Bevy also focuses its canvas during startup, which scrolled the sound button
out of view. The loader restores the starting page position. Clicking the
visible button changed Chromium's Web Audio state from `suspended` to `running`;
this does not verify sound from physical speakers. Node tests cover context
rejection, probe cleanup, scroll restoration, audio-button state, and the
default canvas-size false-positive.

## Follow-up: 2026-10-07

The rendered forest showed its loading overlay because the draw watcher did not
schedule another check after its first WebGL draw. The watcher now polls every
100 ms and resolves after draw activity advances across frames at least 400 ms
apart. The loader races renderer readiness against WASM initialization failure,
so an event loop that keeps its startup promise pending cannot block the ready
state.

Bevy's canvas focus also moved the page to `scrollY = 262`, hiding the sound
button. The route now wraps that canvas instance's `focus()` with
`preventScroll: true` and restores the original method on page unload. Chromium
154 kept the page at `scrollY = 0` with the sound button at viewport position
`y = 215` after startup.

The Chromium 154 sweep loaded all five WebAssembly routes at 1,215 × 700 pixels.
Each reached the ready state after WebGL draw activity, exposed an `Enable
sound` button, and reported two starting audio sources and destination
connections. A trusted click moved each audio context from `suspended` to
`running` and changed the button to `Mute sound`. This checks browser graph
connections and activation; physical speakers were not tested.

The current forest capture contains 36 frames from a 1280 × 720 Chromium 154
viewport using SwiftShader WebGL2. The GIF lasts 4.51 seconds, and the paired
MP4 lasts 4.5 seconds at 8 frames per second. It begins with the direct path
attenuated to 75% low, 65% mid, and 52% high, then shows the listener walk past
the brush screen until all bands reach 100%. One floor-reflection path stays
visible. A trusted click moved the browser audio context from `suspended` to
`running`; speaker output was not tested. The project GIF, MP4 preview, and
WebP poster use only the procedural Bevy scene. They do not reuse Vercidium
video or Patreon images.

The published forest route was also opened in Firefox 157 headless with
`webgl.force-enabled` and `webgl.enable-webgl2`. Firefox returned no WebGL2
context, the canvas stayed at 300 × 150 pixels, and no Bevy audio context was
created. The route showed its Firefox recovery message and kept audio disabled.
The homepage's muted MP4 preview advanced in that same browser. The gallery now
labels the preview as muted and loads the Bevy scene inline after `Run the 3D
demo`; visitors can still open the standalone route. Canvas pointer and
keyboard gestures resume suspended audio after WebGL2 starts.

## References

- [Bevy examples in WebGL2](https://bevy.org/examples/)
- [Vercidium interactive demo](https://www.patreon.com/vercidium/posts/raytraced-audio-155391088)
- [A First Look At Raytraced Audio](https://www.youtube.com/watch?v=u6EuAUjq92k)
- [I Built Raytraced Audio for Godot](https://www.youtube.com/watch?v=A6bPUXTlic8)
- [Vercidium Audio](https://vercidium.com/)
- [Firefox performance settings](https://support.mozilla.org/en-US/kb/performance-settings)
