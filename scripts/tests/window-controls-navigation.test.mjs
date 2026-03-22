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

test("window controls drawer defines the approved route widgets", async () => {
  const drawerSource = await readProjectFile("src/components/layout/WindowControlsDrawer.jsx");

  assert.match(drawerSource, /useNavigate/, "expected route navigation in the drawer");
  assert.match(drawerSource, /useLocation/, "expected route awareness in the drawer");
  assert.match(drawerSource, /clearCareer/, "expected menu widget to clear the active career");
  assert.match(
    drawerSource,
    /emoji:\s*"⚙️"[\s\S]*route:\s*"\/settings"[\s\S]*title:\s*"Configurações"/,
    "expected a settings widget",
  );
  assert.match(
    drawerSource,
    /emoji:\s*"📂"[\s\S]*route:\s*"\/load-save"[\s\S]*title:\s*"Carregar save"/,
    "expected a load-save widget",
  );
  assert.match(
    drawerSource,
    /emoji:\s*"🏠"[\s\S]*route:\s*"\/menu"[\s\S]*title:\s*"Menu principal"/,
    "expected a menu widget",
  );
});

test("window controls drawer becomes global and dashboard menu button is removed", async () => {
  const appSource = await readProjectFile("src/App.jsx");
  const layoutSource = await readProjectFile("src/components/layout/MainLayout.jsx");
  const headerSource = await readProjectFile("src/components/layout/Header.jsx");

  assert.match(
    appSource,
    /import WindowControlsDrawer from "\.\/components\/layout\/WindowControlsDrawer";/,
    "expected App.jsx to import the global drawer",
  );
  assert.match(
    appSource,
    /<WindowControlsDrawer \/>/,
    "expected App.jsx to render the global drawer",
  );
  assert.doesNotMatch(
    layoutSource,
    /<WindowControlsDrawer \/>/,
    "expected MainLayout.jsx not to render a duplicate drawer",
  );
  assert.doesNotMatch(
    headerSource,
    /Voltar ao menu/,
    "expected the redundant dashboard menu button to be removed",
  );
});
