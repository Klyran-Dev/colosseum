import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "@playwright/test";
import { captureStyleSnapshot } from "./styleSnapshot";

/**
 * Loads markup captured from the unit tests (src/styleFixtureCapture.setup.ts)
 * into the real browser with the real stylesheet, so surfaces that no journey
 * reaches can still be style-snapshotted (ADR-0429). Runs only when
 * STYLE_FIXTURE_INPUT_DIR and STYLE_SNAPSHOT_DIR are both set; otherwise every
 * test here is skipped and normal e2e runs are unaffected.
 */
const inputDirectory = process.env.STYLE_FIXTURE_INPUT_DIR;
const enabled = Boolean(inputDirectory && process.env.STYLE_SNAPSHOT_DIR);

interface Fixture {
  name: string;
  hits: string[];
  html: string;
}

const fixtures: Fixture[] = enabled && inputDirectory
  ? readdirSync(inputDirectory)
      .filter((file) => file.endsWith(".json"))
      .sort()
      .map((file) => JSON.parse(readFileSync(join(inputDirectory, file), "utf8")) as Fixture)
  : [];

test.describe("style fixtures from unit-test markup", () => {
  test.skip(!enabled, "set STYLE_FIXTURE_INPUT_DIR and STYLE_SNAPSHOT_DIR (scripts/web_style_snapshot.sh)");
  // Each fixture is independent; this only takes effect with --workers > 1.
  test.describe.configure({ mode: "parallel" });

  for (const fixture of fixtures) {
    test(`fixture ${fixture.name}`, async ({ page }) => {
      await page.goto("/");
      await page.waitForLoadState("networkidle");
      await page.evaluate((html: string) => {
        const alreadyShell = /class="[^"]*\bproduct-shell\b/.test(html);
        document.body.innerHTML = alreadyShell
          ? html
          : `<div class="app-shell product-shell" data-theme="system" dir="ltr"><aside class="app-sidebar" aria-hidden="true"></aside><div class="app-main-column"><main class="content product-content" id="main-content">${html}</main></div></div>`;
      }, fixture.html);
      await captureStyleSnapshot(page, "fixtures", fixture.name);
    });
  }
});
