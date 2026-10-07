import {
  canvasHasBeenResized,
  audioControlState,
  probeWebGL2,
  restoreScrollPosition,
} from "./web-demo-support.mjs";

const startButton = document.querySelector("#start-demo");
const statusLine = document.querySelector("#demo-status");
const canvas = document.querySelector("#bevy-canvas");
const canvasState = document.querySelector("#canvas-state");
const bevyAudioContexts = new Set();

window.__bevyAudioContexts = bevyAudioContexts;

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
  const state = audioControlState([...bevyAudioContexts].map((context) => context.state));
  startButton.disabled = state.disabled;
  startButton.textContent = state.buttonText;
  statusLine.textContent = state.statusText;
}

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
  try {
    if (!probeWebGL2(document)) {
      throw new Error("This browser could not create a WebGL2 canvas.");
    }

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

    await wasmBindings.default({ module_or_path: bytes });
    await waitForCanvasResize(15_000);
  } finally {
    restoreScrollPosition(window, initialScrollPosition);
  }
}

loadBevyScene().then(() => {
  canvasState.hidden = true;
  updateAudioControls();
}).catch((error) => {
  const reason = error instanceof Error ? error.message : String(error);
  const isGraphicsFailure = /WebGL2|renderer did not size/u.test(reason);
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
