import assert from "node:assert/strict";
import test from "node:test";
import { createInteractiveDemoFrame } from "../web-demo-support.mjs";

let importNumber = 0;

function installDocument(reducedMotion = false) {
  const cards = [
    { textContent: "Minimal 2D tutorial", dataset: { search: "wall listener" }, hidden: false },
    { textContent: "Stress test 3D", dataset: { search: "sixteen emitters triangles" }, hidden: false },
  ];
  const sections = [
    { hidden: false, querySelectorAll: () => [cards[0]] },
    { hidden: false, querySelectorAll: () => [cards[1]] },
  ];
  const listeners = new Map();
  const search = {
    value: "",
    tagName: "INPUT",
    addEventListener: (name, listener) => listeners.set(`search:${name}`, listener),
    focus() { document.activeElement = search; },
  };
  const count = { textContent: "" };
  const emptyState = { hidden: true };
  const featuredDescription = { textContent: "The preview is muted." };
  const featuredMeta = { textContent: "Click Enable sound or the scene." };
  const forestPreview = {
    playCalls: 0,
    pauseCalls: 0,
    play() {
      this.playCalls += 1;
      return Promise.resolve();
    },
    pause() { this.pauseCalls += 1; },
  };
  const motionListeners = new Map();
  const launchListeners = new Map();
  const createdFrames = [];
  const featuredArt = {
    children: [],
    classes: [],
    querySelector: () => forestPreview,
    replaceChildren(...children) { this.children = children; },
    classList: { add: (name) => featuredArt.classes.push(name) },
  };
  const launchButton = {
    dataset: {
      demoSrc: "./examples/forest-3d/embed.html",
      demoTitle: "Interactive forest audio demo",
    },
    addEventListener: (name, listener) => launchListeners.set(name, listener),
  };
  const motionPreference = {
    matches: reducedMotion,
    addEventListener: (name, listener) => motionListeners.set(name, listener),
  };
  const document = {
    activeElement: { tagName: "BODY" },
    querySelector(selector) {
      return selector === "#example-search" ? search
        : selector === "#example-count" ? count
          : selector === "[data-motion-preview]" ? forestPreview
            : selector === "#launch-forest-demo" ? launchButton
              : selector === "#featured-art" ? featuredArt
                : selector === "#featured-description" ? featuredDescription
                  : selector === "#featured-meta" ? featuredMeta
          : emptyState;
    },
    querySelectorAll(selector) {
      return selector === "[data-example-section]" ? sections : cards;
    },
    createElement(tagName) {
      const frame = { tagName };
      createdFrames.push(frame);
      return frame;
    },
    addEventListener: (name, listener) => listeners.set(`document:${name}`, listener),
  };

  globalThis.document = document;
  globalThis.window = { matchMedia: () => motionPreference };
  return {
    cards,
    count,
    createdFrames,
    emptyState,
    featuredArt,
    featuredDescription,
    featuredMeta,
    forestPreview,
    launchButton,
    launchListeners,
    listeners,
    motionListeners,
    motionPreference,
    search,
    sections,
  };
}

async function loadCatalog() {
  importNumber += 1;
  await import(`../catalog.mjs?test=${importNumber}`);
}

test("filters example labels and search terms without case sensitivity", async () => {
  const harness = installDocument();
  try {
    await loadCatalog();
    assert.equal(harness.count.textContent, "2 examples");

    harness.search.value = "TRIANGLES";
    harness.listeners.get("search:input")();
    assert.deepEqual(harness.cards.map((card) => card.hidden), [true, false]);
    assert.deepEqual(harness.sections.map((section) => section.hidden), [true, false]);
    assert.equal(harness.count.textContent, "1 example");
    assert.equal(harness.emptyState.hidden, true);
  } finally {
    delete globalThis.document;
    delete globalThis.window;
  }
});

test("shows an empty message when no card matches", async () => {
  const harness = installDocument();
  try {
    await loadCatalog();
    harness.search.value = "no matching example";
    harness.listeners.get("search:input")();
    assert.equal(harness.count.textContent, "0 examples");
    assert.equal(harness.cards.every((card) => card.hidden), true);
    assert.deepEqual(harness.sections.map((section) => section.hidden), [true, true]);
    assert.equal(harness.emptyState.hidden, false);
  } finally {
    delete globalThis.document;
    delete globalThis.window;
  }
});

test("replaces the muted preview with the interactive forest demo on request", async () => {
  const harness = installDocument();
  try {
    await loadCatalog();
    harness.launchListeners.get("click")();

    assert.equal(harness.forestPreview.pauseCalls, 1);
    assert.equal(harness.createdFrames.length, 1);
    assert.deepEqual(harness.featuredArt.children, [harness.createdFrames[0]]);
    assert.equal(harness.createdFrames[0].src, "./examples/forest-3d/embed.html");
    assert.equal(harness.createdFrames[0].title, "Interactive forest audio demo");
    assert.equal(harness.createdFrames[0].loading, "eager");
    assert.equal(harness.createdFrames[0].allow, "autoplay; fullscreen; gamepad");
    assert.deepEqual(harness.featuredArt.classes, ["demo-loaded"]);
    assert.doesNotMatch(harness.featuredDescription.textContent, /preview/u);
    assert.match(harness.featuredDescription.textContent, /direct path clears/u);
    assert.doesNotMatch(harness.featuredMeta.textContent, /Enable sound/u);
    assert.match(harness.featuredMeta.textContent, /arrow keys/u);
  } finally {
    delete globalThis.document;
    delete globalThis.window;
  }
});

test("slash focuses search unless the user is typing in a field", async () => {
  const harness = installDocument();
  try {
    await loadCatalog();
    let prevented = false;
    harness.listeners.get("document:keydown")({
      key: "/",
      ctrlKey: false,
      metaKey: false,
      altKey: false,
      preventDefault: () => { prevented = true; },
    });
    assert.equal(prevented, true);
    assert.equal(globalThis.document.activeElement, harness.search);

    globalThis.document.activeElement = { tagName: "TEXTAREA" };
    prevented = false;
    harness.listeners.get("document:keydown")({
      key: "/",
      ctrlKey: false,
      metaKey: false,
      altKey: false,
      preventDefault: () => { prevented = true; },
    });
    assert.equal(prevented, false);
  } finally {
    delete globalThis.document;
    delete globalThis.window;
  }
});

test("keeps the forest preview paused when reduced motion is enabled", async () => {
  const harness = installDocument(true);
  try {
    await loadCatalog();
    assert.equal(harness.forestPreview.playCalls, 0);
    assert.equal(harness.forestPreview.pauseCalls, 1);
  } finally {
    delete globalThis.document;
    delete globalThis.window;
  }
});

test("stops and resumes the forest preview when the motion preference changes", async () => {
  const harness = installDocument();
  try {
    await loadCatalog();
    assert.equal(harness.forestPreview.playCalls, 1);
    assert.equal(harness.forestPreview.pauseCalls, 0);

    harness.motionPreference.matches = true;
    harness.motionListeners.get("change")();
    assert.equal(harness.forestPreview.pauseCalls, 1);

    harness.motionPreference.matches = false;
    harness.motionListeners.get("change")();
    assert.equal(harness.forestPreview.playCalls, 2);
  } finally {
    delete globalThis.document;
    delete globalThis.window;
  }
});
