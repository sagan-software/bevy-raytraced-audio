# Asset provenance

The examples use generated geometry and one locally generated audio file. The
README GIFs are captures of the browser example pages. No external asset or
third-party sound file is bundled.

| Path | Contents | License and provenance |
| --- | --- | --- |
| `assets/audio/bevy-raytraced-audio-chime.wav` | 2-second mono PCM chime, 44.1 kHz, 16-bit. | Generated locally for this project; no external creator or license applies. SHA-256: `04d9e0bb1a906761e38f598084184830588221065a308fdc80ba3e91b11c0ec4`. |
| `assets/gifs/examples-2d.gif` | Browser page captures for the minimal and stress 2D routes. | Recorded locally from the project's own WebAssembly examples; no external assets are included. |
| `assets/gifs/examples-3d.gif` | Browser page captures for the minimal and stress 3D routes. | Recorded locally from the project's own WebAssembly examples; no external assets are included. |

The browser builder copies the same WAV file beside each example route. It does
not fetch audio or geometry at runtime.
