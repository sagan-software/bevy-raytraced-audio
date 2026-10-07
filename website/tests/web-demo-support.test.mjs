import assert from "node:assert/strict";
import test from "node:test";

import {
  audioControlState,
  canvasHasBeenResized,
  installWebGLDrawMonitor,
  probeWebGL2,
  restoreScrollPosition,
  suppressCanvasFocusScroll,
  waitForCanvasDrawActivity,
  waitForRendererReady,
} from "../web-demo-support.mjs";

test("accepts a rendered canvas while Bevy keeps its WASM startup promise pending", async () => {
  const wasmInitialization = new Promise(() => {});

  await assert.doesNotReject(waitForRendererReady(wasmInitialization, Promise.resolve()));
});

test("reports WASM initialization errors before the canvas renders", async () => {
  const wasmInitialization = Promise.reject(new Error("WebAssembly initialization failed"));

  await assert.rejects(
    waitForRendererReady(wasmInitialization, new Promise(() => {})),
    /WebAssembly initialization failed/u,
  );
});

test("keeps waiting for the rendered canvas when WASM initialization resolves first", async () => {
  let canvasReady = false;
  let resolveCanvas;
  const rendererReady = new Promise((resolve) => {
    resolveCanvas = () => {
      canvasReady = true;
      resolve();
    };
  });

  const waiting = waitForRendererReady(Promise.resolve(), rendererReady);
  await Promise.resolve();
  assert.equal(canvasReady, false);

  resolveCanvas();
  await waiting;
  assert.equal(canvasReady, true);
});

test("waits for successive WebGL draw calls after the first frame", async () => {
  let drawCallCount = 1;
  let now = 0;
  const scheduledChecks = [];
  const monitor = {
    get available() { return true; },
    get drawCallCount() { return drawCallCount; },
  };
  const ready = waitForCanvasDrawActivity(monitor, 1_000, {
    now: () => now,
    schedule: (callback) => scheduledChecks.push(callback),
  });

  assert.equal(scheduledChecks.length, 1);
  drawCallCount = 2;
  now = 400;
  scheduledChecks.shift()();

  await assert.doesNotReject(ready);
});

test("prevents Bevy canvas focus from scrolling its audio controls out of view", () => {
  let focusOptions;
  const canvas = {
    focus(options) {
      focusOptions = options;
    },
  };

  const restore = suppressCanvasFocusScroll(canvas);
  canvas.focus();

  assert.deepEqual(focusOptions, { preventScroll: true });
  restore();
  assert.equal(Object.hasOwn(canvas, "focus"), true);
});

test("keeps caller focus options while preventing canvas scrolling", () => {
  let focusOptions;
  const canvas = {
    focus(options) {
      focusOptions = options;
    },
  };

  const restore = suppressCanvasFocusScroll(canvas);
  canvas.focus({ focusVisible: true, preventScroll: false });

  assert.deepEqual(focusOptions, { focusVisible: true, preventScroll: true });
  restore();
});

test("preserves a later focus override when the canvas wrapper is removed", () => {
  const canvas = { focus() {} };
  const replacement = () => "replacement";
  const restore = suppressCanvasFocusScroll(canvas);

  canvas.focus = replacement;
  restore();

  assert.equal(canvas.focus, replacement);
});

test("leaves canvas focus unchanged when the browser protects its method", () => {
  const canvas = Object.freeze({ focus: () => "native" });
  const restore = suppressCanvasFocusScroll(canvas);

  assert.equal(canvas.focus(), "native");
  assert.doesNotThrow(restore);
});

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

test("tracks Bevy WebGL draw calls and restores the original methods", () => {
  const calls = [];
  const contextPrototype = {
    drawElements(mode, count, type, offset) {
      calls.push([this, mode, count, type, offset]);
      return "drawn";
    },
  };
  const originalDrawElements = contextPrototype.drawElements;
  const context = {};
  const monitor = installWebGLDrawMonitor(contextPrototype);

  assert.equal(contextPrototype.drawElements.call(context, 4, 3, 5123, 0), "drawn");
  assert.equal(monitor.drawCallCount, 1);
  assert.deepEqual(calls, [[context, 4, 3, 5123, 0]]);

  monitor.restore();
  assert.equal(contextPrototype.drawElements, originalDrawElements);
});

test("leaves WebGL prototypes without draw methods unchanged", () => {
  const contextPrototype = { clear() {} };
  const originalClear = contextPrototype.clear;
  const monitor = installWebGLDrawMonitor(contextPrototype);

  assert.equal(monitor.drawCallCount, 0);
  monitor.restore();
  assert.equal(contextPrototype.clear, originalClear);
});

test("continues when browser draw methods are protected or unavailable", () => {
  const protectedPrototype = Object.freeze({ drawArrays() {} });
  const protectedMonitor = installWebGLDrawMonitor(protectedPrototype);
  const missingMonitor = installWebGLDrawMonitor(null);

  assert.equal(protectedMonitor.available, false);
  assert.equal(protectedMonitor.drawCallCount, 0);
  assert.equal(missingMonitor.available, false);
  assert.equal(missingMonitor.drawCallCount, 0);
  protectedMonitor.restore();
  missingMonitor.restore();
});

test("preserves a draw method another script replaces before monitor cleanup", () => {
  const contextPrototype = { drawArrays() {} };
  const replacement = () => "replaced";
  const monitor = installWebGLDrawMonitor(contextPrototype);

  contextPrototype.drawArrays = replacement;
  monitor.restore();

  assert.equal(contextPrototype.drawArrays, replacement);
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

test("does not claim sound is audible while no Bevy source reaches the output", () => {
  assert.deepEqual(audioControlState(["running"], { startedSources: 0, outputConnections: 0 }), {
    disabled: true,
    buttonText: "Audio output unavailable",
    statusText: "The browser audio context is running, but no Bevy sound reached its output. Reload the example to retry.",
  });
});

test("offers mute after a Bevy source connects to a running browser output", () => {
  assert.deepEqual(audioControlState(["running", "running"], { startedSources: 1, outputConnections: 1 }), {
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
