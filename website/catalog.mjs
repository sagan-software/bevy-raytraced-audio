import { createInteractiveDemoFrame } from "./web-demo-support.mjs";

const search = document.querySelector("#example-search");
const cards = [...document.querySelectorAll(".example-card")];
const sections = [...document.querySelectorAll("[data-example-section]")];
const count = document.querySelector("#example-count");
const emptyState = document.querySelector("#empty-search");

function updateExamples() {
  const query = search.value.trim().toLocaleLowerCase();
  let visibleCount = 0;

  for (const card of cards) {
    const searchableText = `${card.textContent} ${card.dataset.search}`.toLocaleLowerCase();
    const matches = searchableText.includes(query);
    card.hidden = !matches;
    visibleCount += Number(matches);
  }

  for (const section of sections) {
    section.hidden = ![...section.querySelectorAll(".example-card")].some((card) => !card.hidden);
  }

  count.textContent = `${visibleCount} ${visibleCount === 1 ? "example" : "examples"}`;
  emptyState.hidden = visibleCount !== 0;
}

search.addEventListener("input", updateExamples);
document.addEventListener("keydown", (event) => {
  if (event.key === "/" && !event.ctrlKey && !event.metaKey && !event.altKey) {
    if (document.activeElement !== search && !["INPUT", "TEXTAREA"].includes(document.activeElement?.tagName)) {
      event.preventDefault();
      search.focus();
    }
  }
});

updateExamples();

const launchFeaturedDemo = document.querySelector("#launch-featured-demo");
const featuredArt = document.querySelector("#featured-art");
const featuredDescription = document.querySelector("#featured-description");
const featuredMeta = document.querySelector("#featured-meta");
if (launchFeaturedDemo && featuredArt) {
  launchFeaturedDemo.addEventListener("click", () => {
    createInteractiveDemoFrame(
      document,
      featuredArt,
      launchFeaturedDemo.dataset.demoSrc,
      launchFeaturedDemo.dataset.demoTitle,
    );
    if (featuredDescription && launchFeaturedDemo.dataset.runningDescription) {
      featuredDescription.textContent = launchFeaturedDemo.dataset.runningDescription;
    }
    if (featuredMeta && launchFeaturedDemo.dataset.runningMeta) {
      featuredMeta.textContent = launchFeaturedDemo.dataset.runningMeta;
    }
  });
}

const featuredPreview = document.querySelector("[data-motion-preview]");
if (
  typeof featuredPreview?.play === "function" && typeof window !== "undefined"
  && typeof window.matchMedia === "function"
) {
  const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
  const updatePreviewMotion = () => {
    if (reducedMotion.matches) {
      featuredPreview.pause();
      return;
    }

    void featuredPreview.play().catch(() => {});
  };

  updatePreviewMotion();
  reducedMotion.addEventListener("change", updatePreviewMotion);
}
