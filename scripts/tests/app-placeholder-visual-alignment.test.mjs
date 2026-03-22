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

test("placeholder component provides page and embedded variants", async () => {
  const placeholderSource = await readProjectFile("src/components/ui/AppPlaceholder.jsx");

  assert.match(
    placeholderSource,
    /embedded = false/,
    "expected placeholder component to support embedded usage",
  );
  assert.match(
    placeholderSource,
    /app-shell/,
    "expected placeholder page variant to use the app shell",
  );
  assert.match(
    placeholderSource,
    /glass-strong/,
    "expected placeholder embedded variant to use a glass panel",
  );
});

test("placeholder pages and tabs reuse the shared placeholder component", async () => {
  const files = [
    "src/pages/Settings.jsx",
    "src/pages/history/Archive.jsx",
    "src/pages/history/Rivalries.jsx",
    "src/pages/history/SeasonsHistory.jsx",
    "src/pages/history/TrophyRoom.jsx",
    "src/pages/tabs/DriversTab.jsx",
    "src/pages/tabs/MarketTab.jsx",
    "src/pages/tabs/MyProfileTab.jsx",
    "src/pages/tabs/NewsTab.jsx",
    "src/pages/tabs/OtherCategoriesTab.jsx",
    "src/pages/tabs/PredictionTab.jsx",
  ];

  for (const file of files) {
    const source = await readProjectFile(file);

    assert.match(source, /AppPlaceholder/, `expected ${file} to use AppPlaceholder`);
  }
});
