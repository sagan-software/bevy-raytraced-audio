import {
  audioControlState,
  canvasHasBeenResized,
  installAudioGestureResume,
  installPointerLockFallback,
  installWebGLDrawMonitor,
  probeWebGL2,
  restoreScrollPosition,
  suppressCanvasFocusScroll,
  waitForCanvasDrawActivity,
  waitForRendererReady,
} from "./web-demo-support.mjs";

const startButton = document.querySelector("#start-demo");
const statusLine = document.querySelector("#demo-status");
const canvas = document.querySelector("#bevy-canvas");
const canvasState = document.querySelector("#canvas-state");
const bevyAudioContexts = new Set();
const bevyAudioGraph = { startedSources: 0, outputConnections: 0 };
let pointerLockUnavailable = false;

window.__bevyAudioContexts = bevyAudioContexts;
window.__bevyAudioGraph = bevyAudioGraph;

function monitorAudioGraph() {
  const sourcePrototype = window.AudioBufferSourceNode?.prototype;
  const originalStart = sourcePrototype?.start;
  if (typeof originalStart === "function") {
    try {
      Object.defineProperty(sourcePrototype, "start", {
        configurable: true,
        writable: true,
        value: function start(...arguments_) {
          const result = Reflect.apply(originalStart, this, arguments_);
          bevyAudioGraph.startedSources += 1;
          updateAudioControls();
          return result;
        },
      });
    } catch {
      // Playback remains available when a browser protects this prototype.
    }
  }

  const nodePrototype = window.AudioNode?.prototype;
  const originalConnect = nodePrototype?.connect;
  if (typeof originalConnect === "function") {
    try {
      Object.defineProperty(nodePrototype, "connect", {
        configurable: true,
        writable: true,
        value: function connect(destination, ...arguments_) {
          const result = Reflect.apply(originalConnect, this, [destination, ...arguments_]);
          if (destination === this.context?.destination) {
            bevyAudioGraph.outputConnections += 1;
            updateAudioControls();
          }
          return result;
        },
      });
    } catch {
      // Playback remains available when a browser protects this prototype.
    }
  }
}

monitorAudioGraph();

const NativeAudioContext = window.AudioContext ?? window.webkitAudioContext;
if (NativeAudioContext) {
  const nativeResume = NativeAudioContext.prototype.resume;
  NativeAudioContext.prototype.resume = function resume(...arguments_) {
    const promise = nativeResume.apply(this, arguments_);
    promise.catch(() => {});
    return promise;
  };

  window.AudioContext = new Proxy(NativeAudioContext, {
    construct(target, arguments_, newTarget) {
      const context = Reflect.construct(target, arguments_, newTarget);
      bevyAudioContexts.add(context);
      context.addEventListener("statechange", updateAudioControls);
      return context;
    },
  });
}

function showCanvasState(message, isError = false) {
  canvasState.hidden = false;
  canvasState.textContent = message;
  canvasState.setAttribute("role", isError ? "alert" : "status");
  statusLine.textContent = message;
}

function browserRecoveryInstructions() {
  if (/Firefox/u.test(navigator.userAgent)) {
    return "In Firefox, open Settings > General > Performance, uncheck Use recommended performance settings, turn on Use hardware acceleration when available, restart Firefox, and reload this page.";
  }

  return "Turn on hardware acceleration in your browser, restart it, then reload this page.";
}

function updateAudioControls() {
  const state = audioControlState(
    [...bevyAudioContexts].map((context) => context.state),
    bevyAudioGraph,
  );
  startButton.disabled = state.disabled;
  startButton.textContent = state.buttonText;
  statusLine.textContent = pointerLockUnavailable
    ? `${state.statusText} Mouse capture is unavailable here: use Q/R to turn and PgUp/PgDn to tilt; F or Esc returns overhead.`
    : state.statusText;
}

const removePointerLockFallback = installPointerLockFallback(canvas, () => {
  pointerLockUnavailable = true;
  updateAudioControls();
});
window.addEventListener("pagehide", removePointerLockFallback, { once: true });

const removeAudioGestureResume = installAudioGestureResume(
  canvas,
  () => bevyAudioContexts,
  updateAudioControls,
);
window.addEventListener("pagehide", removeAudioGestureResume, { once: true });

function waitForCanvasResize(timeoutMilliseconds) {
  const deadline = Date.now() + timeoutMilliseconds;

  return new Promise((resolve, reject) => {
    const checkCanvas = () => {
      if (canvasHasBeenResized(canvas, window.devicePixelRatio)) {
        resolve();
      } else if (Date.now() >= deadline) {
        reject(new Error("The Bevy renderer did not size the browser canvas."));
      } else {
        window.setTimeout(checkCanvas, 100);
      }
    };

    checkCanvas();
  });
}

async function loadBevyScene() {
  const initialScrollPosition = { x: window.scrollX, y: window.scrollY };
  let drawMonitor;
  try {
    if (!probeWebGL2(document)) {
      throw new Error("This browser could not create a WebGL2 canvas.");
    }

    const restoreCanvasFocus = suppressCanvasFocusScroll(canvas);
    window.addEventListener("pagehide", restoreCanvasFocus, { once: true });
    drawMonitor = installWebGLDrawMonitor(window.WebGL2RenderingContext?.prototype);

    const [wasmBindings, bytes] = await Promise.all([
      import(new URL("./pkg/app.js", window.location.href)),
      fetch(new URL("./pkg/app_bg.wasm.gz", window.location.href)).then((response) => {
        if (!response.ok) {
          throw new Error(`The WebAssembly download failed (${response.status}).`);
        }
        if (!("DecompressionStream" in window)) {
          throw new Error("This browser cannot decompress the WebAssembly download.");
        }

        const decompressed = response.body.pipeThrough(new DecompressionStream("gzip"));
        return new Response(decompressed).arrayBuffer();
      }),
    ]);

    const wasmInitialization = wasmBindings.default({ module_or_path: bytes });
    const rendererReady = waitForCanvasResize(15_000)
      .then(() => waitForCanvasDrawActivity(drawMonitor, 15_000));
    await waitForRendererReady(wasmInitialization, rendererReady);
  } finally {
    drawMonitor?.restore();
    restoreScrollPosition(window, initialScrollPosition);
  }
}

loadBevyScene().then(() => {
  canvasState.hidden = true;
  updateAudioControls();
}).catch((error) => {
  const reason = error instanceof Error ? error.message : String(error);
  const isGraphicsFailure = /WebGL2|renderer resized|renderer did not size/u.test(reason);
  const message = isGraphicsFailure
    ? `${reason} ${browserRecoveryInstructions()}`
    : `The Bevy scene failed to start: ${reason}`;
  startButton.disabled = true;
  startButton.textContent = "Scene unavailable";
  showCanvasState(message, true);
});

startButton.addEventListener("click", async () => {
  const contexts = [...bevyAudioContexts];
  if (contexts.length === 0) {
    updateAudioControls();
    return;
  }

  startButton.disabled = true;
  const enableAudio = contexts.some((context) => context.state !== "running");
  const audioOperations = contexts.map((context) => (
    enableAudio ? context.resume() : context.suspend()
  ));
  await Promise.allSettled(audioOperations);
  updateAudioControls();
});
