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

export function restoreScrollPosition(window, position) {
  window.scrollTo(position.x, position.y);
}

export function audioControlState(states) {
  if (states.length === 0) {
    return {
      disabled: true,
      buttonText: "Audio unavailable",
      statusText: "The scene is running, but this browser did not create a Web Audio output.",
    };
  }

  if (states.every((state) => state === "running")) {
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
