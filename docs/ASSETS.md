# Asset provenance

The examples use generated geometry and one locally generated audio file. The
GIFs and forest image are captures of this project's browser scenes. No
third-party scene or audio asset is bundled.

- `assets/audio/bevy-raytraced-audio-chime.wav`: 2-second mono PCM chime at
  44.1 kHz and 16-bit. It was generated locally. SHA-256:
  `04d9e0bb1a906761e38f598084184830588221065a308fdc80ba3e91b11c0ec4`.
- `assets/gifs/examples-2d.gif`: Twelve sampled frames of the 16-emitter,
  32-segment 2D stress scene. It was recorded from this project's WebAssembly
  example on Intel UHD Graphics 620 through ANGLE Vulkan.
- `assets/gifs/examples-3d.gif`: Twelve sampled frames of the 16-emitter,
  32-triangle 3D stress scene. It was recorded from this project's WebAssembly
  example on Intel UHD Graphics 620 through ANGLE Vulkan.
- `website/assets/forest-demo.webp`: A 1215 × 700 capture of the forest route
  on Intel UHD Graphics 620 through ANGLE Vulkan. It is used for the featured
  example and forest card.
- `website/assets/gifs/forest-walk.gif`: Twenty-four frames at 8 FPS from the
  forest route on Intel UHD Graphics 620 through ANGLE Vulkan. The sequence
  moves across the brush screen and back; direct response and band gains change.

The browser builder copies the same WAV file beside each example route. It does
not fetch audio or geometry at runtime.

The website uses the Fira Sans regular and medium web fonts hosted by the Bevy
website. The font files are licensed under the SIL Open Font License 1.1; the
license text is included at `website/assets/fonts/OFL.txt`. The catalogue
illustrations in `website/assets/` are original SVGs made for this project.
The forest WebP and GIF show the project's own procedural scene.

The example package enables Bevy's optional `wav` decoder for this fixture. The
`audio_fixture` integration test decodes the file through Bevy's `Decodable`
implementation so an unsupported audio format fails the test.
