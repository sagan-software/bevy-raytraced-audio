#!/usr/bin/env node
/**
 * Generates the landing page and every example page from `example-catalog.mjs`.
 *
 * Usage: node website/generate-pages.mjs <output-dir> [--allow-missing-sources]
 *
 * The Rust source of each example is read at generation time. A missing source
 * fails the run unless `--allow-missing-sources` is given, in which case the
 * page shows a placeholder instead of the code.
 */

import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  AUDIO_CREDITS_PATH,
  CATEGORIES,
  EXAMPLES,
  featuredExample,
  REPOSITORY_BRANCH,
  REPOSITORY_URL,
  sourceUrl,
} from "./example-catalog.mjs";
import { escapeHtml, highlightRust } from "./rust-highlight.mjs";

const AUDIO_CREDITS_DOCS = "docs/ASSETS.html#third-party-audio-recordings";
const AUDIO_CREDITS_URL = `${REPOSITORY_URL}/blob/${REPOSITORY_BRANCH}/${AUDIO_CREDITS_PATH}`;

const html = escapeHtml;

function categoryOf(categoryId, categories = CATEGORIES) {
  const category = categories.find((candidate) => candidate.id === categoryId);
  if (!category) {
    throw new Error(`Unknown example category: ${categoryId}`);
  }
  return category;
}

function fileName(example) {
  return example.sourcePath.split("/").at(-1);
}

function siteHeader(root, currentPage) {
  const current = (page) => (page === currentPage ? " aria-current=\"page\"" : "");
  return `    <header class="site-header">
      <a class="wordmark" href="${root}" aria-label="Bevy Raytraced Audio examples home">
        <span class="wordmark-mark" aria-hidden="true">B</span>
        <span>Bevy Raytraced Audio</span>
      </a>
      <nav aria-label="Main navigation">
        <a${current("examples")} href="${root}">Examples</a>
        <a href="${root}docs/">Documentation</a>
        <a href="${REPOSITORY_URL}">GitHub</a>
      </nav>
    </header>`;
}

function siteFooter(root) {
  return `    <footer class="site-footer">
      <span>Bevy Raytraced Audio</span>
      <span class="footer-links">
        <a href="${root}${AUDIO_CREDITS_DOCS}">Audio credits</a>
        <a href="${REPOSITORY_URL}">Source and issue tracker</a>
      </span>
    </footer>`;
}

function documentHead({ root, title, description }) {
  return `  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <meta name="description" content="${html(description)}" />
    <title>${html(title)}</title>
    <link rel="icon" type="image/svg+xml" href="${root}assets/favicon.svg" />
    <link rel="stylesheet" href="${root}site.css" />
  </head>`;
}

/** The toolbar and canvas that `web-demo.js` drives. */
function demoFrame(example) {
  return `<section class="demo-frame" aria-label="Interactive demo: ${html(example.title)}">
        <div class="demo-toolbar">
          <button class="start-button" id="start-demo" type="button" disabled>Loading scene…</button>
          <p class="demo-status" id="demo-status" role="status" aria-live="polite">Loading the Bevy scene and checking browser audio.</p>
        </div>
        <div class="canvas-wrap">
          <canvas id="bevy-canvas" aria-label="${html(example.canvasLabel)}" tabindex="0"></canvas>
          <p class="canvas-state" id="canvas-state" role="status" aria-live="polite">Loading the Bevy renderer…</p>
        </div>
      </section>`;
}

function renderKeys(keys) {
  return keys.map((key) => `<kbd>${html(key)}</kbd>`).join(" ");
}

function renderControls(example) {
  const rows = example.controls.map((control) => `
              <div><dt>${renderKeys(control.keys)}</dt><dd>${html(control.action)}</dd></div>`).join("");
  const intro = "<p>Select <strong>Enable sound</strong> or click the scene to start audio.</p>";
  if (rows === "") {
    return `<section aria-labelledby="controls-title">
            <h2 id="controls-title">Controls</h2>
            ${intro}
            <p>This scene runs on its own; no input is needed.</p>
          </section>`;
  }
  return `<section aria-labelledby="controls-title">
            <h2 id="controls-title">Controls</h2>
            ${intro}
            <dl class="controls-list">${rows}
            </dl>
          </section>`;
}

function renderLookFor(example) {
  if (example.lookFor.length === 0) {
    return "";
  }
  const items = example.lookFor.map((item) => `
              <li>${html(item)}</li>`).join("");
  return `<section aria-labelledby="look-for-title">
            <h2 id="look-for-title">What to look for</h2>
            <ul class="look-for-list">${items}
            </ul>
          </section>`;
}

function renderSource(example, source) {
  const name = fileName(example);
  const code = typeof source === "string"
    ? `<pre class="source-code" tabindex="0" aria-label="Rust source of ${html(name)}"><code class="language-rust">${
      highlightRust(source)
    }</code></pre>`
    : `<p class="source-missing">The source of ${html(name)} was not available when this page was generated. <a href="${
      html(sourceUrl(example))
    }">Read it on GitHub</a>.</p>`;
  return `<section class="source-section" id="source" aria-labelledby="source-title">
        <div class="source-header">
          <div>
            <h2 id="source-title">Source code</h2>
            <code class="source-path">${html(example.sourcePath)}</code>
          </div>
          <div class="source-actions">
            <button class="copy-source" type="button" hidden>Copy</button>
            <a class="source-github" href="${html(sourceUrl(example))}">View on GitHub</a>
          </div>
        </div>
        ${code}
      </section>`;
}

function renderPager(example, examples) {
  const index = examples.indexOf(example);
  const previous = examples[index - 1];
  const next = examples[index + 1];
  const link = (target, rel, label) => (target
    ? `<a class="pager-${rel}" rel="${rel}" href="../${html(target.slug)}/"><span>${label}</span>${
      html(target.title)
    }</a>`
    : "<span></span>");
  return `<nav class="example-pager" aria-label="More examples">
        ${link(previous, "prev", "Previous")}
        ${link(next, "next", "Next")}
      </nav>`;
}

/**
 * Renders a standalone example page: demo, description, controls, and source.
 * @param {import("./example-catalog.mjs").Example} example
 * @param {{ source: string | null, examples?: object[], categories?: object[] }} options
 */
export function renderExamplePage(example, { source, examples = EXAMPLES, categories = CATEGORIES }) {
  const root = "../../";
  const category = categoryOf(example.category, categories);
  const description = example.description.map((paragraph) => `
          <p>${html(paragraph)}</p>`).join("");
  return `<!doctype html>
<html lang="en">
${documentHead({ root, title: `${example.title} · Bevy Raytraced Audio`, description: example.summary })}
  <body>
${siteHeader(root, "examples")}
    <main class="demo-main example-page">
      <div class="example-breadcrumbs">
        <nav aria-label="Breadcrumb">
          <ol>
            <li><a href="${root}">Examples</a></li>
            <li><a href="${root}#category-${html(category.id)}">${html(category.label)}</a></li>
            <li><span aria-current="page">${html(example.title)}</span></li>
          </ol>
        </nav>
        <a href="${html(sourceUrl(example))}">View on GitHub</a>
      </div>
      ${demoFrame(example)}
      <article class="example-article">
        <header class="example-intro">
          <p class="example-label">${html(example.label)}</p>
          <h1>${html(example.title)}</h1>
          <p class="example-lead">${html(example.summary)}</p>
        </header>
        <div class="example-body">
          <div class="example-description">${description}
          <p class="example-credits">Sounds are CC0 field and foley recordings. See the <a href="${root}${AUDIO_CREDITS_DOCS}">audio credits</a> or <a href="${
    html(AUDIO_CREDITS_URL)
  }">CREDITS.md</a>.</p>
          </div>
          <aside class="example-facts" aria-label="Controls and tips">
          ${renderControls(example)}
          ${renderLookFor(example)}
          </aside>
        </div>
      </article>
      ${renderSource(example, source)}
      ${renderPager(example, examples)}
    </main>
${siteFooter(root)}
    <script type="module" src="${root}web-demo.js"></script>
    <script type="module" src="${root}source-view.mjs"></script>
  </body>
</html>
`;
}

/** Renders the compact page that the landing page loads in an iframe. */
export function renderEmbedPage(example) {
  const root = "../../";
  return `<!doctype html>
<html lang="en">
${documentHead({ root, title: `${example.title} · Bevy Raytraced Audio`, description: example.summary })}
  <body class="demo-embed">
    <main class="demo-main embed-main">
      <div class="demo-heading">
        <h1>${html(example.title)}</h1>
        <span class="demo-label">${html(example.label)}</span>
      </div>
      ${demoFrame(example)}
    </main>
    <script type="module" src="${root}web-demo.js"></script>
  </body>
</html>
`;
}

function renderCard(example) {
  return `
              <a class="example-card" href="./examples/${html(example.slug)}/" data-search="${
    html(example.searchTerms)
  }">
                <img class="card-image" src="./${html(example.cardImage)}" alt="${html(example.cardAlt)}" />
                <div class="card-copy"><h3>${html(example.title)}</h3><p>${html(example.summary)}</p></div>
              </a>`;
}

function shortControls(example, limit) {
  return example.controls.slice(0, limit)
    .map((control) => `${control.keys.join(" / ")} ${control.action.toLocaleLowerCase()}`)
    .join(" · ");
}

/** Renders the landing page with the featured demo and one section per category. */
export function renderIndexPage({ examples = EXAMPLES, categories = CATEGORIES } = {}) {
  const featured = featuredExample(examples);
  const sections = categories
    .map((category) => ({ category, members: examples.filter((example) => example.category === category.id) }))
    .filter(({ members }) => members.length > 0);
  const nav = sections.map(({ category }) => `
            <a href="#category-${html(category.id)}">${html(category.navLabel)}</a>`).join("");
  const sectionHtml = sections.map(({ category, members }) => `
          <section class="examples-section" id="category-${
    html(category.id)
  }" data-example-section aria-labelledby="heading-${html(category.id)}">
            <div class="section-heading"><h2 id="heading-${html(category.id)}">${html(category.label)}</h2></div>
            <div class="example-grid">${members.map(renderCard).join("")}
            </div>
          </section>`).join("");
  const runningDescription = featured.description[0];
  const runningMeta = shortControls(featured, 4);

  return `<!doctype html>
<html lang="en">
${
    documentHead({
      root: "./",
      title: "Examples · Bevy Raytraced Audio",
      description: "Browse interactive Bevy ray-traced audio examples for 2D and 3D games.",
    })
  }
  <body>
${siteHeader("./", "examples")}

    <main class="landing">
      <section class="catalog-heading" aria-labelledby="page-title">
        <div>
          <h1 id="page-title">Examples</h1>
          <p class="catalog-intro">Run each Bevy 0.19 scene in your browser, then read its full source and controls.</p>
        </div>
        <label class="search-field">
          <span class="sr-only">Search examples</span>
          <svg class="search-icon" aria-hidden="true" viewBox="0 0 20 20" fill="none">
            <circle cx="8.75" cy="8.75" r="5.5" stroke="currentColor" stroke-width="1.5" />
            <path d="m13 13 4 4" stroke="currentColor" stroke-linecap="round" stroke-width="1.5" />
          </svg>
          <input id="example-search" type="search" placeholder="Search examples" autocomplete="off" />
          <kbd>/</kbd>
        </label>
      </section>

      <div class="catalog-layout">
        <aside class="catalog-nav" aria-label="Example categories">
          <p class="nav-label">Audio</p>
          <nav>
            <a href="#featured">Featured demo</a>${nav}
          </nav>
        </aside>

        <div class="catalog-content">
          <section class="featured-example" id="featured" aria-labelledby="featured-title">
            <div class="featured-art" id="featured-art">
              <img class="featured-preview" src="./${html(featured.cardImage)}" alt="${html(featured.cardAlt)}" />
              <button
                class="launch-demo"
                id="launch-featured-demo"
                type="button"
                data-demo-src="./examples/${html(featured.slug)}/embed.html"
                data-demo-title="Interactive demo: ${html(featured.title)}"
                data-running-description="${html(runningDescription)}"
                data-running-meta="${html(runningMeta)}"
              >
                <svg aria-hidden="true" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z" /></svg>
                <span>Run the demo</span>
              </button>
            </div>
            <div class="featured-copy">
              <h2 id="featured-title">${html(featured.title)}</h2>
              <p id="featured-description">${
    html(featured.summary)
  } Run the Bevy scene here, or open the full page for the controls and source.</p>
              <a class="text-link" href="./examples/${html(featured.slug)}/">Open the example page</a>
              <p class="featured-meta" id="featured-meta">Click Enable sound or the scene to start audio</p>
            </div>
          </section>

          <div class="section-heading catalog-count">
            <h2>Audio examples</h2>
            <p id="example-count" aria-live="polite">${examples.length} examples</p>
          </div>
${sectionHtml}

          <p class="empty-search" id="empty-search" hidden>No examples match that search.</p>
          <section class="capability-note" aria-labelledby="capability-title">
            <div class="capability-mark" aria-hidden="true">CPU</div>
            <div>
              <h2 id="capability-title">About the audio model</h2>
              <p>Rays leave each listener on the CPU. Rays that reach a source drive occlusion and muffling, rays that return drive echo and reverb, and rays that escape drive outdoor ambience. Sounds are CC0 recordings; see the <a href="./${AUDIO_CREDITS_DOCS}">audio credits</a>.</p>
            </div>
            <a href="./docs/">Read the book</a>
          </section>
        </div>
      </div>
    </main>

${siteFooter("./")}
    <script type="module" src="./catalog.mjs"></script>
  </body>
</html>
`;
}

/**
 * Reads example sources and writes the landing page, example pages, and embeds.
 * @param {{ outputDir: string, repositoryRoot: string, allowMissingSources?: boolean, examples?: object[], categories?: object[] }} options
 * @returns {Promise<{ written: string[], missingSources: string[] }>}
 */
export async function generateSite({
  outputDir,
  repositoryRoot,
  allowMissingSources = false,
  examples = EXAMPLES,
  categories = CATEGORIES,
}) {
  const slugs = new Set();
  for (const example of examples) {
    if (slugs.has(example.slug)) {
      throw new Error(`Duplicate example slug: ${example.slug}`);
    }
    slugs.add(example.slug);
    categoryOf(example.category, categories);
  }

  const written = [];
  const missingSources = [];
  const write = async (relativePath, contents) => {
    const target = path.join(outputDir, relativePath);
    await mkdir(path.dirname(target), { recursive: true });
    await writeFile(target, contents);
    written.push(relativePath);
  };

  const sources = await Promise.all(examples.map(async (example) => {
    try {
      const entryPath = path.join(repositoryRoot, example.sourcePath);
      const entry = await readFile(entryPath, "utf8");
      const modules = [...entry.matchAll(/#\[path = "([^"\n]+)"\]/gu)];
      const children = await Promise.all(modules.map(async ([, relative]) => {
        const modulePath = path.resolve(path.dirname(entryPath), relative);
        if (!modulePath.startsWith(`${path.dirname(entryPath)}${path.sep}`)) {
          throw new Error(`Example module escapes its source directory: ${relative}`);
        }
        return `\n\n// ── ${relative} ──\n${await readFile(modulePath, "utf8")}`;
      }));
      return entry + children.join("");
    } catch (error) {
      if (error?.code !== "ENOENT") {
        throw error;
      }
      missingSources.push(example.sourcePath);
      return null;
    }
  }));

  if (missingSources.length > 0 && !allowMissingSources) {
    throw new Error(`Missing example sources: ${missingSources.join(", ")}`);
  }

  await write("index.html", renderIndexPage({ examples, categories }));
  for (const [index, example] of examples.entries()) {
    await write(
      `examples/${example.slug}/index.html`,
      renderExamplePage(example, { source: sources[index], examples, categories }),
    );
    await write(`examples/${example.slug}/embed.html`, renderEmbedPage(example));
  }

  return { written, missingSources };
}

async function main(arguments_) {
  const allowMissingSources = arguments_.includes("--allow-missing-sources");
  const positional = arguments_.filter((argument) => !argument.startsWith("--"));
  if (positional.length !== 1) {
    console.error("Usage: node website/generate-pages.mjs <output-dir> [--allow-missing-sources]");
    process.exitCode = 2;
    return;
  }

  const repositoryRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const { written, missingSources } = await generateSite({
    outputDir: path.resolve(positional[0]),
    repositoryRoot,
    allowMissingSources,
  });
  for (const missing of missingSources) {
    console.warn(`warning: ${missing} is missing; its page shows a placeholder`);
  }
  console.log(`Generated ${written.length} pages in ${positional[0]}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2)).catch((error) => {
    console.error(error instanceof Error ? error.message : error);
    process.exitCode = 1;
  });
}
