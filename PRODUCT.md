# Product

## Confirmed purpose

Bevy Raytraced Audio adds 2D and 3D acoustic path tracing to existing Bevy
projects. Existing Bevy audio playback remains the playback system. The site
must let readers open each runnable example and hear its audio in a browser.

## Audience and use

The primary audience is Bevy developers evaluating the crate through small,
working examples before integrating it into an existing game. The gallery is a
separate experience surface; the Markdown book carries detailed setup and API
guidance.

## Inferred decisions

- The public site uses static HTML, CSS, and JavaScript so GitHub Pages can host
  it without a server runtime.
- The site follows Bevy's example catalogue pattern, with category navigation
  and direct example cards. A procedural forest scene leads the catalogue.
- Each browser example has its own route and WebAssembly build output.
- Each browser demo asks for a user click before resuming audio playback.
- The site checks for WebGL2 before loading a Bevy renderer and reports a
  recovery step when the browser cannot create a WebGL2 context.

## Product limits

The current implementation traces CPU paths and updates Bevy audio sink volume
for direct-path occlusion. GPU tracing, mesh extraction, reflection DSP, and
hardware-independent browser audio behavior are not promised.
