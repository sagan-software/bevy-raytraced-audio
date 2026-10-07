import assert from "node:assert/strict";
import test from "node:test";

let importNumber = 0;

function installDocument() {
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
  const document = {
    activeElement: { tagName: "BODY" },
    querySelector(selector) {
      return selector === "#example-search" ? search
        : selector === "#example-count" ? count
          : emptyState;
    },
    querySelectorAll(selector) {
      return selector === "[data-example-section]" ? sections : cards;
    },
    addEventListener: (name, listener) => listeners.set(`document:${name}`, listener),
  };

  globalThis.document = document;
  return { cards, count, emptyState, listeners, search, sections };
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
  }
});
