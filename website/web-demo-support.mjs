export function probeWebGL2(document) {
  try {
    const canvas = document.createElement("canvas");
    const context = canvas.getContext("webgl2", {
      failIfMajorPerformanceCaveat: false,
    });
    if (!context) {
      return false;
    }

    context.getExtension("WEBGL_lose_context")?.loseContext();
    return true;
  } catch {
    return false;
  }
}

/** Replaces an animated poster with the matching Bevy example after a visitor asks to run it. */
export function createInteractiveDemoFrame(document, container, source, title) {
  container.querySelector?.("video")?.pause();
  const frame = document.createElement("iframe");
  frame.title = title;
  frame.src = source;
  frame.loading = "eager";
  frame.allow = "autoplay; fullscreen; gamepad";
  container.replaceChildren(frame);
  container.classList.add("demo-loaded");
  return frame;
}

/** Resumes browser-suspended Bevy audio when the visitor interacts with its canvas. */
export function installAudioGestureResume(canvas, getContexts, updateControls) {
  const resumeSuspendedContexts = () => {
    const resumeOperations = [...getContexts()]
      .filter((context) => context.state !== "running")
      .map((context) => {
        try {
          return context.resume();
        } catch (error) {
          return Promise.reject(error);
        }
      });

    if (resumeOperations.length > 0) {
      void Promise.allSettled(resumeOperations).then(updateControls);
    }
  };

  canvas.addEventListener("pointerdown", resumeSuspendedContexts);
  canvas.addEventListener("keydown", resumeSuspendedContexts);

  return () => {
    canvas.removeEventListener("pointerdown", resumeSuspendedContexts);
    canvas.removeEventListener("keydown", resumeSuspendedContexts);
  };
}

export function canvasHasBeenResized(canvas, devicePixelRatio = 1) {
  if (
    canvas.clientWidth <= 0
    || canvas.clientHeight <= 0
    || canvas.width <= 0
    || canvas.height <= 0
  ) {
    return false;
  }

  const expectedWidth = Math.round(canvas.clientWidth * devicePixelRatio);
  const expectedHeight = Math.round(canvas.clientHeight * devicePixelRatio);
  return Math.abs(canvas.width - expectedWidth) <= 1
    && Math.abs(canvas.height - expectedHeight) <= 1;
}

/**
 * Waits for visible renderer activity without waiting for Bevy's browser event loop to exit.
 * @param {Promise<unknown>} wasmInitialization WebAssembly initialization and Bevy startup.
 * @param {Promise<unknown>} rendererReady Canvas sizing and successive rendered frames.
 * @returns {Promise<void>} Resolves when the renderer is ready or rejects on startup failure.
 */
export function waitForRendererReady(wasmInitialization, rendererReady) {
  const initializationFailure = wasmInitialization.then(() => new Promise(() => {}));
  return Promise.race([rendererReady, initializationFailure]);
}

/**
 * Waits for multiple WebGL draw calls so one startup frame cannot mark a blank demo ready.
 * @param {{ available: boolean, drawCallCount: number }} drawMonitor WebGL draw counter.
 * @param {number} timeoutMilliseconds Maximum wait for successive draws.
 * @param {{ now?: () => number, schedule?: (callback: () => void, delay: number) => unknown }} clock Timer controls for deterministic tests.
 * @returns {Promise<void>} Resolves after successive draws or rejects when the renderer stalls.
 */
export function waitForCanvasDrawActivity(
  drawMonitor,
  timeoutMilliseconds,
  { now = Date.now, schedule = globalThis.setTimeout } = {},
) {
  if (!drawMonitor.available) {
    return Promise.resolve();
  }

  const deadline = now() + timeoutMilliseconds;

  return new Promise((resolve, reject) => {
    let previousDrawCount = 0;
    let firstDrawAt = null;

    const checkDrawActivity = () => {
      const currentTime = now();
      const currentDrawCount = drawMonitor.drawCallCount;

      if (currentDrawCount > 0 && firstDrawAt === null) {
        previousDrawCount = currentDrawCount;
        firstDrawAt = currentTime;
      }

      if (
        firstDrawAt !== null
        && currentDrawCount > previousDrawCount
        && currentTime - firstDrawAt >= 400
      ) {
        resolve();
      } else if (currentTime >= deadline) {
        reject(new Error("The Bevy renderer resized its canvas but did not draw successive frames."));
      } else {
        schedule(checkDrawActivity, 100);
      }
    };

    checkDrawActivity();
  });
}

/**
 * Counts WebGL draw calls during startup so a resized but blank canvas is not ready.
 * @param {object | null | undefined} contextPrototype WebGL2 context prototype.
 * @returns {{ drawCallCount: number, available: boolean, restore: () => void }}
 */
export function installWebGLDrawMonitor(contextPrototype) {
  let drawCallCount = 0;
  const wrappedMethods = [];
  const methodNames = [
    "drawArrays",
    "drawElements",
    "drawRangeElements",
    "drawArraysInstanced",
    "drawElementsInstanced",
  ];

  if (contextPrototype && (typeof contextPrototype === "object" || typeof contextPrototype === "function")) {
    for (const methodName of methodNames) {
      const originalDescriptor = Object.getOwnPropertyDescriptor(contextPrototype, methodName);
      if (typeof originalDescriptor?.value !== "function") {
        continue;
      }

      const originalMethod = originalDescriptor.value;
      const instrumentedMethod = function instrumentedDraw(...arguments_) {
        drawCallCount += 1;
        return Reflect.apply(originalMethod, this, arguments_);
      };

      try {
        Object.defineProperty(contextPrototype, methodName, {
          ...originalDescriptor,
          value: instrumentedMethod,
        });
        wrappedMethods.push({ methodName, originalDescriptor, instrumentedMethod });
      } catch {
        // A browser may expose a non-writable WebGL prototype method.
      }
    }
  }

  return {
    get drawCallCount() {
      return drawCallCount;
    },
    get available() {
      return wrappedMethods.length > 0;
    },
    restore() {
      for (const { methodName, originalDescriptor, instrumentedMethod } of wrappedMethods) {
        const currentDescriptor = Object.getOwnPropertyDescriptor(contextPrototype, methodName);
        if (currentDescriptor?.value === instrumentedMethod) {
          Object.defineProperty(contextPrototype, methodName, originalDescriptor);
        }
      }
    },
  };
}

export function restoreScrollPosition(window, position) {
  window.scrollTo(position.x, position.y);
}

/**
 * Makes Bevy's canvas focus preserve the page position and its audio controls.
 * @param {object | null | undefined} canvas Bevy's focusable HTML canvas.
 * @returns {() => void} Restores the canvas focus method when the page unloads.
 */
export function suppressCanvasFocusScroll(canvas) {
  if (!canvas || typeof canvas.focus !== "function") {
    return () => {};
  }

  const originalDescriptor = Object.getOwnPropertyDescriptor(canvas, "focus");
  const originalFocus = canvas.focus;
  const focusWithoutScroll = function focus(options) {
    const focusOptions = options && typeof options === "object" ? options : {};
    return Reflect.apply(originalFocus, this, [{ ...focusOptions, preventScroll: true }]);
  };

  try {
    Object.defineProperty(canvas, "focus", {
      configurable: true,
      enumerable: originalDescriptor?.enumerable ?? false,
      writable: true,
      value: focusWithoutScroll,
    });
  } catch {
    return () => {};
  }

  return () => {
    if (canvas.focus !== focusWithoutScroll) {
      return;
    }

    if (originalDescriptor) {
      Object.defineProperty(canvas, "focus", originalDescriptor);
    } else {
      delete canvas.focus;
    }
  };
}

export function audioControlState(states, audioGraph = {}) {
  if (states.length === 0) {
    return {
      disabled: true,
      buttonText: "Audio unavailable",
      statusText: "The scene is running, but this browser did not create a Web Audio output.",
    };
  }

  if (states.every((state) => state === "running")) {
    if (audioGraph.startedSources === 0 || audioGraph.outputConnections === 0) {
      return {
        disabled: true,
        buttonText: "Audio output unavailable",
        statusText: "The browser audio context is running, but no Bevy sound reached its output. Reload the example to retry.",
      };
    }

    return {
      disabled: false,
      buttonText: "Mute sound",
      statusText: "Spatial sound is on. Use WASD to move through the scene.",
    };
  }

  if (states.every((state) => state === "suspended")) {
    return {
      disabled: false,
      buttonText: "Enable sound",
      statusText: "The scene is running. Enable sound to hear the spatial chime.",
    };
  }

  return {
    disabled: false,
    buttonText: "Retry sound",
    statusText: "The browser did not enable every audio output. Select Retry sound.",
  };
}
