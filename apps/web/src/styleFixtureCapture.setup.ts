// Test-only (ADR-0429): when STYLE_FIXTURE_DIR is set, save the real DOM that a
// unit test rendered, the first time each class listed in
// e2e/fixture-targets.json appears during the test and again at its end
// (or every state with STYLE_FIXTURE_ALL=1). scripts/web_style_fixtures.sh runs
// this with `--sequence.hooks=list` so the final capture happens before each
// test file's own cleanup. The saved markup is loaded into the real browser with
// the real stylesheet and snapshotted like any journey state. Inert otherwise.
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { afterEach, beforeEach, expect } from "vitest";

const directory = process.env.STYLE_FIXTURE_DIR;

if (directory) {
  const targets: string[] = JSON.parse(readFileSync(resolve(process.cwd(), "e2e/fixture-targets.json"), "utf8"));
  const everything = Boolean(process.env.STYLE_FIXTURE_ALL);
  let seen = new Set<string>();
  let counter = 0;
  let observer: MutationObserver | undefined;

  const classesIn = (html: string): Set<string> => {
    const present = new Set<string>();
    for (const match of html.matchAll(/class="([^"]*)"/g)) match[1].split(/\s+/).forEach((token) => token && present.add(token));
    return present;
  };
  const save = (suffix: string, hits: string[], html: string): void => {
    const state = expect.getState();
    const file = (state.testPath ?? "unknown").split("/").pop()?.replace(/\.test\.tsx?$/, "") ?? "unknown";
    const name = `${file} ${state.currentTestName ?? ""} ${suffix}`.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "").slice(0, 170);
    mkdirSync(directory, { recursive: true });
    writeFileSync(join(directory, `${name}.json`), JSON.stringify({ name, hits, html }));
  };
  const check = (): void => {
    const html = document.body.innerHTML;
    if (html.length < 40) return;
    const hits = targets.filter((token) => classesIn(html).has(token));
    const fresh = hits.filter((token) => !seen.has(token));
    if (fresh.length === 0) return;
    fresh.forEach((token) => seen.add(token));
    save(`first-${++counter}`, hits, html);
  };

  beforeEach(() => {
    seen = new Set();
    counter = 0;
    if (typeof document === "undefined" || typeof MutationObserver === "undefined") return;
    observer = new MutationObserver(check);
    observer.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ["class"] });
  });

  afterEach(() => {
    observer?.disconnect();
    observer = undefined;
    if (typeof document === "undefined") return;
    const html = document.body.innerHTML;
    if (html.length < 40) return;
    const hits = targets.filter((token) => classesIn(html).has(token));
    if (!everything && hits.length === 0) return;
    save("end", hits, html);
  });
}
