const startButton = document.querySelector("#start-demo");
const statusLine = document.querySelector("#demo-status");

const bevyAudioContexts = new Set();
window.__bevyAudioContexts = bevyAudioContexts;

// Retain Bevy's Web Audio contexts so a user gesture can resume them later.
const NativeAudioContext = window.AudioContext;
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

// Load the scene asynchronously because browsers limit large synchronous WebAssembly modules.
Promise.all([
  import(new URL("./pkg/app.js", window.location.href)),
  fetch(new URL("./pkg/app_bg.wasm.gz", window.location.href)).then((response) => {
    if (!response.ok) {
      throw new Error(`WebAssembly download failed (${response.status}).`);
    }
    if (!("DecompressionStream" in window)) {
      throw new Error("This browser does not support gzip decompression streams.");
    }
    const decompressed = response.body.pipeThrough(new DecompressionStream("gzip"));
    return new Response(decompressed).arrayBuffer();
  }),
]).then(async ([wasmBindings, bytes]) => {
  await wasmBindings.default({ module_or_path: bytes });
  // Bevy may focus its canvas during startup, which can scroll the audio control away.
  await new Promise((resolve) => requestAnimationFrame(resolve));
  window.scrollTo({ top: 0, left: 0, behavior: "instant" });
  startButton.disabled = false;
  startButton.textContent = "Enable spatial audio";
  statusLine.textContent = "Scene ready. Enable audio to hear the spatial sound.";
}).catch((error) => {
  statusLine.textContent = `This browser demo could not load: ${error.message}`;
});

startButton.addEventListener("click", () => {
  const contexts = [...bevyAudioContexts];
  if (contexts.length === 0) {
    statusLine.textContent = "The scene loaded without a browser audio output.";
    return;
  }

  startButton.disabled = true;
  // Call resume directly from this gesture to satisfy browser audio policy.
  const resumeResults = contexts.map((context) => context.resume());
  Promise.allSettled(resumeResults).then(() => {
    const runningCount = contexts.filter((context) => context.state === "running").length;
    if (runningCount === contexts.length) {
      startButton.textContent = "Spatial audio enabled";
      statusLine.textContent = "Audio is enabled. The Bevy scene is running.";
    } else {
      statusLine.textContent = "The browser did not enable audio output.";
      startButton.disabled = false;
    }
  });
});
