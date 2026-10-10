# Modular village models

All models are by **Kenney**, dedicated to the public domain under
[Creative Commons Zero 1.0](https://creativecommons.org/publicdomain/zero/1.0/).
The original `License.txt` is retained in each pack directory.

| Directory    | Original pack                                              | Used for                                                                 |
| ------------ | ---------------------------------------------------------- | ------------------------------------------------------------------------ |
| `building/`  | [Building Kit](https://kenney.nl/assets/building-kit), 1.0 | Walls, apertures, animated door leaves, floor tiles, stairs and railings |
| `furniture/` | [Furniture Kit](https://kenney.nl/assets/furniture-kit)    | Living room, kitchen, upstairs bedroom and basement furniture            |
| `nature/`    | [Nature Kit](https://kenney.nl/assets/nature-kit)          | Modular bridge, trees and stream-bank rocks                              |

Downloaded from the author's official pack pages on 2026-10-10. Only selected
GLB models are included. Building textures retain their original relative paths.
Furniture and nature models contain their original materials.

`building/wall-window-open.glb` and `building/window-leaf.glb` are derivatives of
`wall-window-square.glb`: the existing glass child was separated from the frame
so the pane can pivot on its original hinge. Geometry, material and texture data
are unchanged. Doors are rotated at the hinge in code; the original animations
remain in the GLB. Double doors scale two leaves to the kit's wide aperture.

Player/NPC humanoids, terrain banks, bridge approaches, and acoustic proxies are
procedural repository geometry. Furniture is decorative; acoustic surfaces are
explicit lightweight proxies for the structural modules, not every visual mesh.
