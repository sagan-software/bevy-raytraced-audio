# Asset provenance

The examples use generated geometry and one locally generated audio file. The
README GIFs are captures of the browser stress scenes. No external asset or
third-party sound file is bundled.

- `assets/audio/bevy-raytraced-audio-chime.wav`: 2-second mono PCM chime at
  44.1 kHz and 16-bit. It was generated locally. SHA-256:
  `04d9e0bb1a906761e38f598084184830588221065a308fdc80ba3e91b11c0ec4`.
- `assets/gifs/examples-2d.gif`: Twelve sampled frames of the 16-emitter,
  32-segment 2D stress scene. It was recorded from this project's WebAssembly
  example on Intel UHD Graphics 620 through ANGLE Vulkan.
- `assets/gifs/examples-3d.gif`: Twelve sampled frames of the 16-emitter,
  32-triangle 3D stress scene. It was recorded from this project's WebAssembly
  example on Intel UHD Graphics 620 through ANGLE Vulkan.

The browser builder copies the same WAV file beside each example route. It does
not fetch audio or geometry at runtime.

The example package enables Bevy's optional `wav` decoder for this fixture. The
`audio_fixture` integration test decodes the file through Bevy's `Decodable`
implementation so an unsupported audio format fails the test.
