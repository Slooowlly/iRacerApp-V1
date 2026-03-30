import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.resolve(__dirname, "..", "..");

async function readProjectFile(relativePath) {
  return readFile(path.join(projectRoot, relativePath), "utf8");
}

test("dashboard navigation promotes News instead of Next Race", async () => {
  const dashboardSource = await readProjectFile("src/pages/Dashboard.jsx");
  const navSource = await readProjectFile("src/components/layout/TabNavigation.jsx");

  assert.match(dashboardSource, /NewsTab/, "expected Dashboard to render NewsTab");
  assert.doesNotMatch(
    dashboardSource,
    /NextRaceTab/,
    "expected Dashboard to stop importing the dedicated next race tab",
  );
  assert.match(navSource, /label:\s*"Noticias"/, "expected top nav to include Noticias");
  assert.doesNotMatch(
    navSource,
    /Proxima Corrida/,
    "expected top nav to remove Proxima Corrida",
  );
});

test("news tab renders the editorial section hierarchy in order", async () => {
  const source = await readProjectFile("src/pages/tabs/NewsTab.jsx");

  const markers = [
    'data-news-section="hero"',
    'data-news-section="scope-tabs"',
    'data-news-section="primary-filters"',
    'data-news-section="context-filters"',
    'data-news-section="main-reader"',
  ];

  let lastIndex = -1;
  for (const marker of markers) {
    const index = source.indexOf(marker);
    assert.notEqual(index, -1, `expected NewsTab to include ${marker}`);
    assert.ok(index > lastIndex, `expected ${marker} to appear after the previous section`);
    lastIndex = index;
  }

   assert.equal(
    source.includes('data-news-section="cover"'),
    false,
    "expected NewsTab to drop the old cover section",
  );
  assert.equal(
    source.includes('data-news-section="feed"'),
    false,
    "expected NewsTab to drop the old feed section",
  );
});

test("news tab includes the famous scope and reduced special-mode filters", async () => {
  const source = await readProjectFile("src/pages/tabs/NewsTab.jsx");

  assert.match(source, /Mais famosos/, "expected NewsTab to expose the special Mais famosos scope");
  assert.match(
    source,
    /const PRIMARY_FILTER_IDS = \["Corridas", "Pilotos", "Equipes", "Mercado"\]/,
    "expected NewsTab to remove Expectativas from the visible primary filters",
  );
  assert.match(
    source,
    /const FAMOUS_FILTER_IDS = \["Pilotos", "Equipes", "Mercado"\]/,
    "expected NewsTab to define the reduced filter set for the famous scope",
  );
  assert.doesNotMatch(
    source,
    /const PRIMARY_FILTER_IDS = \["Corridas", "Pilotos", "Equipes", "Mercado", "Expectativas"\]/,
    "expected NewsTab to stop defining Expectativas as a top-level filter",
  );
});
