import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.resolve(__dirname, "..", "..");

test("window controls drawer separates hover zone from visual tray", async () => {
  const drawerSource = await readFile(
    path.join(projectRoot, "src/components/layout/WindowControlsDrawer.jsx"),
    "utf8",
  );

  assert.match(
    drawerSource,
    /className="relative h-\[390px\] w-\[148px\]"/,
    "expected a narrower interaction wrapper around the tray",
  );
  assert.match(
    drawerSource,
    /className="absolute -left-\[32px\] top-0 h-\[390px\] w-\[188px\]"/,
    "expected a dedicated invisible hover zone",
  );
  assert.match(
    drawerSource,
    /className="relative z-10 ml-auto w-\[132px\]"/,
    "expected the visual tray to be isolated from the hover zone",
  );
});
