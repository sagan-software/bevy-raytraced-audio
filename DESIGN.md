# Website design

## Structure

The landing page follows Bevy's example catalogue: a category index, search,
category headings, and two-column example cards. A forest scene leads the
catalogue with an animated, muted preview. Selecting its launch control
replaces the preview with the Bevy scene; a standalone route remains available.
The 2D tutorial, 3D tutorial, and stress tests keep separate browser routes.

## Visual system

Use Bevy's charcoal surfaces, Fira Sans, blue-violet links, and restrained
separators. The featured forest uses deep pine and moss, weathered stone, an
amber sound source, a green listener, and teal path lines. Keep catalogue
controls in the site layer and keep path colors visible against the scene.

## Interaction

Each browser route loads a separate Bevy WebAssembly build. Check WebGL2 before
downloading the build, and wait until Bevy resizes its canvas before reporting
the scene as ready. A blocked renderer shows a recovery instruction in the
canvas. The audio button resumes or suspends Bevy's Web Audio context from a
user click. Canvas pointer and keyboard gestures also resume suspended audio.
In the forest scene, WASD moves the listener, mouse drag orbits the camera, the
wheel changes camera distance, B toggles paths, and F1 toggles diagnostics.

## Accessibility

Use links for navigation and native buttons for audio controls. Give every
scene an accessible name, show loading and error text outside the canvas, and
announce status changes through a live region. Keep keyboard instructions
visible and preserve focus styles and responsive layouts.
