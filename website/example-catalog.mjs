/**
 * The single description of every browser example. `generate-pages.mjs` turns
 * this list into the landing page, one page per example, and one embed page
 * per example. The Rust source is read at build time from `sourcePath`.
 */

export const REPOSITORY_URL = "https://github.com/sagan-software/bevy-raytraced-audio";
export const REPOSITORY_BRANCH = "main";
export const AUDIO_CREDITS_PATH = "examples/raytraced-audio/assets/audio/CREDITS.md";

/** Category order and labels for the landing page and breadcrumbs. */
export const CATEGORIES = [
  { id: "showcase", label: "Showcase", navLabel: "Sandbox" },
  { id: "2d", label: "2D", navLabel: "2D" },
  { id: "3d", label: "3D", navLabel: "3D" },
  { id: "stress", label: "Stress tests", navLabel: "Stress tests" },
];

const sourcePath = (name) => `examples/raytraced-audio/examples/${name}.rs`;

/**
 * @typedef {{ keys: string[], action: string }} Control
 * @typedef {{
 *   name: string, slug: string, title: string, category: string, label: string,
 *   featured?: boolean, summary: string, description: string[], controls: Control[],
 *   lookFor: string[], canvasLabel: string, sourcePath: string, cardImage: string,
 *   cardAlt: string, searchTerms: string,
 * }} Example
 */

/** @type {Example[]} */
export const EXAMPLES = [
  {
    name: "showcase",
    slug: "showcase",
    title: "Acoustic village sandbox",
    category: "showcase",
    label: "Showcase · modular houses · 3D acoustics",
    featured: true,
    summary: "Open doors and windows, hear footsteps across floors, and explore a stream and bridge.",
    description: [
      "Explore two furnished houses assembled from Kenney’s CC0 Building and Furniture Kits. The speech cottage has single and double doors, opening windows, and removable wall sections. The other house has a basement, upstairs, and two walking NPCs.",
      "These are real 3D acoustics: walls, floors, ceilings, doors, and panes share their positions with the visible modules. In overhead view, upper stories disappear visually while continuing to affect sound. Walk up the stairs or switch to first person to see inside.",
      "The character faces the mouse in third person. Press F to look through their eyes; blue marks the left ear and coral the right. Listening direction follows the character, independently of the overhead camera.",
      "The numbered presets isolate speech, footsteps above or below you, and the stream from the bridge or water’s edge. Start quietly, then adjust the master level. Tab bypasses filtering and reverb for an A/B comparison. Measured HRTF headphone rendering adds elevation cues; B compares it with ordinary stereo panning. T cycles tile, carpet, and furnishings. J/K plays a clap/gunshot indoors or outdoors. The model does not simulate vibration through floor joists.",
    ],
    controls: [
      { keys: ["F"], action: "Switch between overhead and first-person view" },
      { keys: ["Esc"], action: "Return overhead and release the mouse" },
      { keys: ["Q", "R"], action: "Orbit overhead / turn in first person" },
      { keys: ["W", "A", "S", "D"], action: "Walk relative to the camera; Shift walks faster" },
      { keys: ["Mouse"], action: "Face the cursor overhead / look around in first person" },
      { keys: ["PgUp", "PgDn"], action: "Tilt the first-person view if mouse capture is unavailable" },
      { keys: ["E"], action: "Open or close the nearest door, double doors, or window" },
      { keys: ["O", "C"], action: "Open / close all openings for repeatable comparisons" },
      { keys: ["X"], action: "Remove or rebuild a nearby editable wall section" },
      { keys: ["0", "1–8"], action: "Explore freely / select an isolated listening scenario" },
      { keys: ["Tab"], action: "Bypass room filtering and reverb to compare" },
      { keys: ["B"], action: "Compare measured headphone elevation cues with stereo panning" },
      { keys: ["T"], action: "Cycle empty tile, empty carpet, and furnished carpet" },
      { keys: ["J", "K"], action: "Play a clap / gunshot to compare indoor and outdoor acoustics" },
      { keys: ["V"], action: "Show or hide acoustic rays" },
      { keys: ["P"], action: "Pause or resume the walking NPCs" },
      { keys: ["N"], action: "Toggle rain in exploration mode" },
      { keys: ["G"], action: "Toggle your own footsteps in exploration mode" },
      { keys: ["M"], action: "Mute or unmute everything" },
      { keys: ["−", "+"], action: "Lower or raise master volume" },
      { keys: ["Wheel"], action: "Zoom overhead" },
      { keys: ["H"], action: "Hide or show the in-scene guide and preset buttons" },
    ],
    lookFor: [
      "Choose 1, then E: hear the words brighten as the double doors open. C closes all openings; Tab compares the same scene without processing.",
      "Use stereo headphones, choose 2 or 3, and press B to compare above/below cues. T changes floor and furniture acoustics; J or K repeats the same impulse.",
      "Choose 6 to listen through a window. E opens its pane; the wall frame stays solid.",
      "Choose 2 or 3 to isolate footsteps upstairs or in the basement. Visit those floors with 7/8, or use the two staircases.",
      "Compare 4 and 5: water below the wooden bridge is occluded by the deck, while beside the stream it has a direct path.",
      "Near the front-left side of either house, X removes and rebuilds one wall module. The acoustic opening and visible opening change together.",
    ],
    canvasLabel:
      "Acoustic village with furnished modular houses, opening doors and windows, three floors, NPC footsteps, speech, and a stream with a bridge",
    sourcePath: sourcePath("showcase"),
    cardImage: "assets/cards/showcase.svg",
    cardAlt: "Modular houses beside a stream with acoustic rays connecting listeners and sources",
    searchTerms:
      "showcase sandbox village home house doors windows stairs basement upstairs speech voice stream bridge footsteps rain 3d acoustics",
  },
  {
    name: "minimal_2d",
    slug: "minimal-2d",
    title: "Muffling around a door",
    category: "2d",
    label: "2D · Basics",
    summary: "Open and close a door between you and a speaker and hear the muffling change.",
    description: [
      "You (the blue circle) stand in the left room. A music speaker (the red circle) plays in the right room, behind a thin wall with a door. This is the smallest scene that shows occlusion with the 2D plugin.",
      "Every frame the listener fires sound rays that bounce off the walls; the white lines show them travelling, slowed down so you can watch. When a bounce point can see the speaker, a green line connects them, so more green means a clearer sound. Orange lines go straight through the wall, where the treble is lost.",
      "With the door open, green rays reach the speaker and the music is clear. With the door closed, only orange rays get through the thin wall and only the bass remains.",
    ],
    controls: [
      { keys: ["Space"], action: "Open or close the door" },
      { keys: ["W", "A", "S", "D"], action: "Move the listener (arrow keys also work)" },
      { keys: ["V"], action: "Show or hide rays" },
    ],
    lookFor: [
      "Green rays appear as soon as the door opens.",
      "With the door closed, the music turns dull and bassy.",
      "Walk around the room to see how line of sight changes the green rays.",
    ],
    canvasLabel: "Top-down 2D scene with a listener, a wall with a door, and a music speaker",
    sourcePath: sourcePath("minimal_2d"),
    cardImage: "assets/cards/minimal-2d.svg",
    cardAlt: "Top-down scene with a listener, a door in a wall, and a speaker",
    searchTerms: "2d minimal basics tutorial door occlusion muffling wall transmission listener",
  },
  {
    name: "reverb_2d",
    slug: "reverb-2d",
    title: "Echo and room size",
    category: "2d",
    label: "2D · Reverb",
    summary: "Clap in a bathroom, a hall, and a cathedral and compare the echo.",
    description: [
      "Blue rays leave the listener, bounce around the room, and return. Their travel times and remaining energy set the reverb: how long it lasts (RT60) and how long before the first echo arrives (pre-delay).",
      "Switch between three rooms of different sizes, then add clutter to see the furniture break up the echo paths.",
    ],
    controls: [
      { keys: ["1"], action: "Bathroom" },
      { keys: ["2"], action: "Hall" },
      { keys: ["3"], action: "Cathedral" },
      { keys: ["C"], action: "Toggle clutter that blocks echo rays" },
      { keys: ["Space"], action: "Clap" },
    ],
    lookFor: [
      "The RT60 and pre-delay readout grows with room size.",
      "Clutter blocks blue echo rays, so the reverb gets shorter and drier.",
      "A clap in the cathedral rings much longer than one in the bathroom.",
    ],
    canvasLabel: "Top-down 2D room with a listener, blue echo rays, and an RT60 and pre-delay readout",
    sourcePath: sourcePath("reverb_2d"),
    cardImage: "assets/cards/reverb-2d.svg",
    cardAlt: "Top-down room with blue echo rays bouncing between the walls",
    searchTerms: "2d reverb echo rt60 pre-delay room size bathroom hall cathedral clutter clap",
  },
  {
    name: "ambience_2d",
    slug: "ambience-2d",
    title: "Indoor, outdoor, and rain direction",
    category: "2d",
    label: "2D · Ambience",
    summary: "Walk in and out of a house and hear the rain come through its openings.",
    description: [
      "Rays that escape to the open sky tell the plugin how outdoor the listener is. Outside, the rain is loud and all around you; inside, it gets quieter.",
      "Yellow rays point the rain toward the open windows and doors, so indoors the rain comes from the openings rather than from everywhere.",
    ],
    controls: [
      { keys: ["W", "A", "S", "D"], action: "Move the listener" },
      { keys: ["1", "2", "3"], action: "Toggle the door, west window, and north window" },
      { keys: ["B"], action: "Show the bounces too" },
    ],
    lookFor: [
      "Step through the door and listen to the rain move from all around you to the doorway.",
      "Close every opening and the rain becomes a quiet, muffled patter.",
      "The share of escaped rays drops as you walk deeper into the house.",
    ],
    canvasLabel: "Top-down 2D house with windows and a door, the listener, and yellow ambience rays",
    sourcePath: sourcePath("ambience_2d"),
    cardImage: "assets/cards/ambience-2d.svg",
    cardAlt: "Top-down house with yellow rays leaving through a window toward the rain",
    searchTerms: "2d ambience rain outdoor indoor escaped rays windows door weather",
  },
  {
    name: "permeation_2d",
    slug: "permeation-2d",
    title: "Wall thickness",
    category: "2d",
    label: "2D · Permeation",
    summary: "Compare a speaker behind a thin wooden wall and a thick stone wall.",
    description: [
      "Orange rays pass straight through walls and lose energy at every wall face they cross. Each material absorbs the frequency bands differently.",
      "The speaker moves between a thin wooden room and a thick stone room. The readout shows how much energy reaches the listener in each band.",
    ],
    controls: [
      { keys: ["Tab"], action: "Move the speaker to the other room now" },
    ],
    lookFor: [
      "Through wood, the music is quieter but still clear.",
      "Through stone, the high bands nearly disappear and the music is heavily muffled.",
      "Count how many wall faces each orange ray crosses.",
    ],
    canvasLabel: "Top-down 2D scene with a wooden room, a stone room, a moving speaker, and orange permeation rays",
    sourcePath: sourcePath("permeation_2d"),
    cardImage: "assets/cards/permeation-2d.svg",
    cardAlt: "Orange rays crossing a thin wooden wall and a thick stone wall",
    searchTerms: "2d permeation transmission wall thickness wood stone materials frequency bands muffling",
  },
  {
    name: "minimal_3d",
    slug: "minimal-3d",
    title: "3D room with a doorway",
    category: "3d",
    label: "3D · Basics",
    summary: "The 3D plugin on a small room: hear a source clear up through a doorway.",
    description: [
      "The smallest 3D scene: a room with a doorway, a listener, and a sound source. It shows how to set up the 3D plugin and mark listeners, emitters, and acoustic surfaces.",
      "When the doorway lines up between you and the source, the direct path clears and the sound brightens. Behind the walls, the sound is muffled.",
    ],
    controls: [
      { keys: ["Space"], action: "Open or close the door" },
      { keys: ["V"], action: "Show or hide rays" },
      { keys: ["W", "A", "S", "D"], action: "Move the listener (arrow keys also work)" },
    ],
    lookFor: [
      "The sound clears when you can see the source through the doorway.",
      "Echo rays bounce off the floor and walls inside the room.",
    ],
    canvasLabel: "3D room with a doorway, a listener, a sound source, and acoustic rays",
    sourcePath: sourcePath("minimal_3d"),
    cardImage: "assets/cards/minimal-3d.svg",
    cardAlt: "Simple 3D room with a doorway and sound paths",
    searchTerms: "3d minimal basics tutorial room doorway spatial audio occlusion",
  },
  {
    name: "forest_3d",
    slug: "forest-3d",
    title: "Forest sound paths",
    category: "3d",
    label: "3D · Interactive",
    summary: "Walk a forest around a stone arch and watch direct and reflected sound paths.",
    description: [
      "Walk a small low-poly forest while a sound source moves past a stone arch and a brush screen. The scene uses shared low-detail meshes and a fixed set of explicit acoustic triangles.",
      "The stone arch and brush screen use different acoustic materials, so the sound changes as it passes behind each one.",
    ],
    controls: [
      { keys: ["W", "A", "S", "D"], action: "Move the listener (arrow keys also work)" },
      { keys: ["Shift"], action: "Move faster" },
      { keys: ["Hold left mouse"], action: "Orbit the camera" },
      { keys: ["Wheel"], action: "Zoom" },
      { keys: ["B"], action: "Show or hide paths" },
      { keys: ["F1"], action: "Show or hide diagnostics" },
      { keys: ["Esc"], action: "Release the cursor" },
    ],
    lookFor: [
      "The direct path clears as you step out from behind the arch.",
      "Stone blocks more sound than the brush screen.",
    ],
    canvasLabel: "Low-poly forest with a stone arch, a walking listener, a moving sound source, and acoustic paths",
    sourcePath: sourcePath("forest_3d"),
    cardImage: "assets/forest-demo.webp",
    cardAlt: "Low-poly forest with a listener, a sound source, a stone arch, and path lines",
    searchTerms: "3d forest interactive spatial reflection paths materials stone brush arch",
  },
  {
    name: "stress_2d",
    slug: "stress-2d",
    title: "16 sources in 2D",
    category: "stress",
    label: "2D · Stress test",
    summary: "Trace 16 moving emitters against 32 wall segments.",
    description: [
      "Exercises the 2D adapter with 16 moving sources and 32 explicit wall segments. The on-screen readout shows the frame rate.",
    ],
    controls: [],
    lookFor: [
      "The frame rate with every emitter traced each frame.",
    ],
    canvasLabel: "Top-down 2D scene with 16 moving sound sources and 32 wall segments",
    sourcePath: sourcePath("stress_2d"),
    cardImage: "assets/cards/stress-2d.svg",
    cardAlt: "Top-down stress scene with many moving sources and wall segments",
    searchTerms: "2d stress performance 16 emitters 32 walls benchmark",
  },
  {
    name: "stress_3d",
    slug: "stress-3d",
    title: "16 sources in 3D",
    category: "stress",
    label: "3D · Stress test",
    summary: "Trace 16 moving emitters against 32 wall triangles.",
    description: [
      "Exercises the 3D adapter with 16 moving sources and 32 explicit triangles. The on-screen readout shows the frame rate.",
    ],
    controls: [],
    lookFor: [
      "The frame rate with every emitter traced each frame.",
    ],
    canvasLabel: "3D scene with 16 moving sound sources and 32 wall triangles",
    sourcePath: sourcePath("stress_3d"),
    cardImage: "assets/cards/stress-3d.svg",
    cardAlt: "3D stress scene with many moving sources and surfaces",
    searchTerms: "3d stress performance 16 emitters 32 triangles benchmark",
  },
];

export function featuredExample(examples = EXAMPLES) {
  return examples.find((example) => example.featured) ?? examples[0];
}

export function sourceUrl(example) {
  return `${REPOSITORY_URL}/blob/${REPOSITORY_BRANCH}/${example.sourcePath}`;
}
