import assert from "node:assert/strict";
import test from "node:test";

import {
  audioControlState,
  canvasHasBeenResized,
  probeWebGL2,
  restoreScrollPosition,
} from "../web-demo-support.mjs";

test("reports when the browser cannot create a WebGL2 context", () => {
  const document = {
    createElement: () => ({ getContext: () => null }),
  };

  assert.equal(probeWebGL2(document), false);
});

test("reports when WebGL2 probing throws", () => {
  const document = {
    createElement: () => ({
      getContext: () => { throw new Error("WebGL2 is blocked"); },
    }),
  };

  assert.equal(probeWebGL2(document), false);
});

test("releases a WebGL2 probe context after a successful check", () => {
  let contextReleased = false;
  const document = {
    createElement: () => ({
      getContext: (kind, attributes) => {
        assert.equal(kind, "webgl2");
        assert.deepEqual(attributes, { failIfMajorPerformanceCaveat: false });
        return {
          getExtension: (name) => {
            assert.equal(name, "WEBGL_lose_context");
            return { loseContext: () => { contextReleased = true; } };
          },
        };
      },
    }),
  };

  assert.equal(probeWebGL2(document), true);
  assert.equal(contextReleased, true);
});

test("keeps the demo in its loading state while the canvas has default dimensions", () => {
  assert.equal(
    canvasHasBeenResized({ width: 300, height: 150, clientWidth: 1134, clientHeight: 638 }, 1),
    false,
  );
});

test("does not accept a canvas when only one backing dimension changed", () => {
  assert.equal(
    canvasHasBeenResized({ width: 1134, height: 150, clientWidth: 1134, clientHeight: 638 }, 1),
    false,
  );
});

test("accepts the renderer canvas after Bevy sizes it to the display", () => {
  assert.equal(
    canvasHasBeenResized({ width: 1134, height: 638, clientWidth: 1134, clientHeight: 638 }, 1),
    true,
  );
});

test("accepts small mobile canvases and high-density displays", () => {
  assert.equal(
    canvasHasBeenResized({ width: 288, height: 300, clientWidth: 288, clientHeight: 300 }, 1),
    true,
  );
  assert.equal(
    canvasHasBeenResized({ width: 576, height: 600, clientWidth: 288, clientHeight: 300 }, 2),
    true,
  );
});

test("rejects a hidden or zero-sized canvas", () => {
  assert.equal(canvasHasBeenResized({ width: 1280, height: 720, clientWidth: 0, clientHeight: 0 }), false);
  assert.equal(canvasHasBeenResized({ width: 0, height: 0, clientWidth: 320, clientHeight: 300 }), false);
});

test("restores the page position after the renderer focuses its canvas", () => {
  const scrollCalls = [];
  const window = { scrollTo: (...position) => scrollCalls.push(position) };

  restoreScrollPosition(window, { x: 0, y: 24 });

  assert.deepEqual(scrollCalls, [[0, 24]]);
});

test("labels the audio control when browser audio is unavailable", () => {
  assert.deepEqual(audioControlState([]), {
    disabled: true,
    buttonText: "Audio unavailable",
    statusText: "The scene is running, but this browser did not create a Web Audio output.",
  });
});

test("labels the audio control when all browser outputs are already running", () => {
  assert.deepEqual(audioControlState(["running", "running"]), {
    disabled: false,
    buttonText: "Mute sound",
    statusText: "Spatial sound is on. Use WASD to move through the scene.",
  });
});

test("labels the audio control when browser outputs await a user gesture", () => {
  assert.deepEqual(audioControlState(["suspended"]), {
    disabled: false,
    buttonText: "Enable sound",
    statusText: "The scene is running. Enable sound to hear the spatial chime.",
  });
});

test("offers a retry when browser audio outputs have mixed states", () => {
  assert.deepEqual(audioControlState(["running", "suspended"]), {
    disabled: false,
    buttonText: "Retry sound",
    statusText: "The browser did not enable every audio output. Select Retry sound.",
  });
});
