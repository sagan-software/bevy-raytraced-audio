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
- Each of the four native examples has its own browser route and WebAssembly
  build output.
- Each browser demo starts from a user click. This starts Bevy and requests
  audio playback under the browser's interaction policy.
- The site uses a dark, low-distraction presentation with the examples and
  links visible on first load.

## Product limits

The current implementation traces CPU paths and updates Bevy audio sink volume
for direct-path occlusion. GPU tracing, mesh extraction, reflection DSP, and
hardware-independent browser audio behavior are not promised.
