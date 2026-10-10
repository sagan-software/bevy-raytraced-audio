import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { CATEGORIES, EXAMPLES, featuredExample, sourceUrl } from "../example-catalog.mjs";
import { generateSite, renderEmbedPage, renderExamplePage, renderIndexPage } from "../generate-pages.mjs";

const websiteDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repositoryRoot = path.resolve(websiteDir, "..");

test("the catalogue lists the nine examples in order with unique slugs", () => {
  assert.deepEqual(EXAMPLES.map((example) => example.name), [
    "showcase",
    "minimal_2d",
    "reverb_2d",
    "ambience_2d",
    "permeation_2d",
    "minimal_3d",
    "forest_3d",
    "stress_2d",
    "stress_3d",
  ]);
  assert.equal(new Set(EXAMPLES.map((example) => example.slug)).size, EXAMPLES.length);
  assert.equal(featuredExample().slug, "showcase");
  for (const example of EXAMPLES) {
    assert.ok(CATEGORIES.some((category) => category.id === example.category), example.name);
    assert.equal(example.sourcePath, `examples/raytraced-audio/examples/${example.name}.rs`);
    assert.equal(example.slug, example.name.replaceAll("_", "-"));
    assert.ok(example.title && example.summary && example.canvasLabel && example.cardAlt, example.name);
  }
});

test("the build script builds the same examples as the catalogue", async () => {
  const script = await readFile(path.join(repositoryRoot, "scripts/build-web-site.sh"), "utf8");
  const list = (name) => script.match(new RegExp(`^${name}=\\(([^)]*)\\)`, "mu"))[1].trim().split(/\s+/u);
  assert.deepEqual(list("example_names"), EXAMPLES.map((example) => example.name));
  assert.deepEqual(list("example_slugs"), EXAMPLES.map((example) => example.slug));
  assert.match(script, /assets\/audio\/music_loop\.ogg/u);
  assert.match(script, /examples\/raytraced-audio\/assets/u);
});

test("every card image exists", async () => {
  for (const example of EXAMPLES) {
    await assert.doesNotReject(readFile(path.join(websiteDir, example.cardImage)), example.cardImage);
  }
});

test("example pages put the demo first, then the description, then highlighted source", () => {
  const example = EXAMPLES.find((candidate) => candidate.slug === "showcase");
  const page = renderExamplePage(example, { source: "fn main() { println!(\"<hi>\"); }\n" });

  const canvas = page.indexOf("id=\"bevy-canvas\"");
  const title = page.indexOf(`<h1>${example.title}</h1>`);
  const code = page.indexOf("<pre class=\"source-code\"");
  assert.ok(canvas > 0 && canvas < title && title < code);

  for (const id of ["start-demo", "demo-status", "bevy-canvas", "canvas-state"]) {
    assert.match(page, new RegExp(`id="${id}"`, "u"));
  }
  assert.match(page, /<code class="language-rust"><span class="tok-keyword">fn<\/span>/u);
  assert.match(page, /&quot;&lt;hi&gt;&quot;/u);
  assert.doesNotMatch(page, /"<hi>"/u);
  assert.ok(page.includes(`href="${sourceUrl(example)}"`));
  assert.match(page, /<kbd>Tab<\/kbd>/u);
  assert.match(page, /What to look for/u);
  assert.match(page, /Audio credits/u);
  assert.match(page, /src="\.\.\/\.\.\/web-demo\.js"/u);
  assert.match(page, /rel="next" href="\.\.\/minimal-2d\/"/u);
});

test("an example page without a source links to GitHub instead", () => {
  const example = EXAMPLES.find((candidate) => candidate.slug === "reverb-2d");
  const page = renderExamplePage(example, { source: null });
  assert.doesNotMatch(page, /<pre class="source-code"/u);
  assert.match(page, /was not available/u);
});

test("embed pages keep the demo controls without site chrome", () => {
  const page = renderEmbedPage(EXAMPLES[0]);
  assert.match(page, /<body class="demo-embed">/u);
  assert.match(page, /id="start-demo"/u);
  assert.match(page, /id="bevy-canvas"/u);
  assert.doesNotMatch(page, /site-header/u);
});

test("the landing page features the showcase and has a card for every example", () => {
  const page = renderIndexPage();
  assert.match(page, /id="launch-featured-demo"/u);
  assert.match(page, /data-demo-src="\.\/examples\/showcase\/embed\.html"/u);
  assert.match(page, /data-running-meta="[^"]*first-person/u);
  for (const example of EXAMPLES) {
    assert.ok(page.includes(`href="./examples/${example.slug}/" data-search=`), example.slug);
  }
  for (const category of CATEGORIES) {
    assert.match(page, new RegExp(`id="category-${category.id}" data-example-section`, "u"));
  }
  assert.match(page, new RegExp(`${EXAMPLES.length} examples`, "u"));
  assert.match(page, /id="example-search"/u);
  assert.match(page, /id="empty-search"/u);
});

test("generateSite writes every page and reports missing sources", async () => {
  const fakeRoot = await mkdtemp(path.join(tmpdir(), "raytraced-audio-src-"));
  const outputDir = await mkdtemp(path.join(tmpdir(), "raytraced-audio-site-"));
  try {
    const present = EXAMPLES.slice(0, 2);
    for (const example of present) {
      await mkdir(path.dirname(path.join(fakeRoot, example.sourcePath)), { recursive: true });
      await writeFile(path.join(fakeRoot, example.sourcePath), `// ${example.name}\nfn main() {}\n`);
    }

    await assert.rejects(
      generateSite({ outputDir, repositoryRoot: fakeRoot }),
      /Missing example sources: .*reverb_2d\.rs/u,
    );

    const { written, missingSources } = await generateSite({
      outputDir,
      repositoryRoot: fakeRoot,
      allowMissingSources: true,
    });
    assert.equal(written.length, 1 + EXAMPLES.length * 2);
    assert.equal(missingSources.length, EXAMPLES.length - present.length);
    const showcase = await readFile(path.join(outputDir, "examples/showcase/index.html"), "utf8");
    assert.match(showcase, /<span class="tok-comment">\/\/ showcase<\/span>/u);
    await assert.doesNotReject(readFile(path.join(outputDir, "examples/stress-3d/embed.html")));
    await assert.doesNotReject(readFile(path.join(outputDir, "index.html")));
  } finally {
    await rm(fakeRoot, { recursive: true, force: true });
    await rm(outputDir, { recursive: true, force: true });
  }
});

test("generated source includes each local Rust module beside the example entry point", async () => {
  const fakeRoot = await mkdtemp(path.join(tmpdir(), "raytraced-module-src-"));
  const outputDir = await mkdtemp(path.join(tmpdir(), "raytraced-module-site-"));
  const example = EXAMPLES[0];
  try {
    const directory = path.dirname(path.join(fakeRoot, example.sourcePath));
    await mkdir(path.join(directory, "parts"), { recursive: true });
    await writeFile(
      path.join(fakeRoot, example.sourcePath),
      "#[path = \"parts/world.rs\"]\nmod world;\nfn main() {}\n",
    );
    await writeFile(path.join(directory, "parts/world.rs"), "// actual modular world source\n");
    await generateSite({ outputDir, repositoryRoot: fakeRoot, examples: [example] });
    const page = await readFile(path.join(outputDir, "examples/showcase/index.html"), "utf8");
    assert.match(page, /parts\/world\.rs/u);
    assert.match(page, /actual modular world source/u);
  } finally {
    await rm(fakeRoot, { recursive: true, force: true });
    await rm(outputDir, { recursive: true, force: true });
  }
});
