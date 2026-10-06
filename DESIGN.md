# Website design

## Structure

The landing page presents four directly linked Bevy scenes: minimal 2D,
minimal 3D, 2D stress, and 3D stress. Every scene has a separate route with a
large canvas, an audio start control, and links to the other scenes.

## Visual system

The site uses a near-black blue-gray background, warm white text, and green and
amber accents that match listener, emitter, and acoustic surface colors in the
demos. System fonts avoid external requests. A two-column gallery collapses to
one column on narrow screens.

## Interaction

The browser downloads and compiles each page's WebAssembly module before it
enables the start button. The click handler resumes Bevy's audio context from
the user's gesture. The demo page reports loading failures next to the disabled
control.

## Accessibility

All navigation uses links. Start controls use native buttons, visible focus
styles, and text status. The canvas has a descriptive accessible name. Text and
controls remain usable at narrow viewport widths.
