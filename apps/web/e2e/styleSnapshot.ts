import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import type { Page, TestType } from "@playwright/test";

/**
 * Opt-in computed-style snapshots for CSS refactors (ADR-0428).
 *
 * When STYLE_SNAPSHOT_DIR is set, every passing journey records the computed
 * style and geometry of every element in the page state it ends in, in the real
 * dark and light themes with transitions disabled. Diffing two directories
 * proves a stylesheet change is visually neutral (or lists exactly what it
 * changed) for surfaces that only render with mocked API data. Without the
 * variable this registers nothing, so normal runs are unaffected.
 */
const PROPS = [
  "color", "backgroundColor", "borderTopColor", "borderRightColor", "borderBottomColor", "borderLeftColor",
  "borderTopWidth", "borderLeftWidth", "borderTopLeftRadius", "borderTopRightRadius", "borderBottomLeftRadius",
  "borderBottomRightRadius", "boxShadow", "fontSize", "fontWeight", "lineHeight", "letterSpacing", "textTransform",
  "paddingTop", "paddingRight", "paddingBottom", "paddingLeft", "marginTop", "marginRight", "marginBottom",
  "marginLeft", "display", "opacity", "gridTemplateColumns", "gap", "position", "textDecorationLine", "outlineStyle",
] as const;

interface SnapshotTheme {
  keys: string[];
  ids: number[];
  strs: string[];
}

/** Bump when the recorded properties or capture semantics change; the diff tool refuses to compare across versions. */
export const SNAPSHOT_FORMAT = 3;

interface Snapshot {
  format: number;
  settled: boolean;
  title: string;
  url: string;
  classes: string[];
  dark: SnapshotTheme;
  light: SnapshotTheme;
}

function fileName(title: string): string {
  return `${title.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "").slice(0, 200)}.json`;
}

async function capture(page: Page, selector = ".product-shell, .product-shell *"): Promise<Omit<Snapshot, "title" | "url" | "format" | "settled"> | null> {
  const captured = await page.evaluate(async ({ props, selector: scope }: { props: string[]; selector: string }) => {
    const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
    if (!document.getElementById("__style_snapshot_nt")) {
      const style = document.createElement("style");
      style.id = "__style_snapshot_nt";
      style.textContent = "*,*::before,*::after{transition:none !important;animation:none !important}";
      document.head.appendChild(style);
    }
    const shell = document.querySelector(".product-shell");
    if (!shell) return null;
    const classes = new Set<string>();
    const snapshotTheme = () => {
      const elements = [...document.querySelectorAll(scope)];
      const strs = new Map<string, number>();
      const ids: number[] = [];
      const keys: string[] = [];
      for (const element of elements) {
        const computed = getComputedStyle(element);
        const box = element.getBoundingClientRect();
        const text = props.map((prop: string) => (computed as unknown as Record<string, string>)[prop]).join("|") + `|${Math.round(box.width)}x${Math.round(box.height)}`;
        if (!strs.has(text)) strs.set(text, strs.size);
        ids.push(strs.get(text) as number);
        const tokens = (element.getAttribute("class") ?? "").split(/\s+/).filter(Boolean);
        tokens.forEach((token) => classes.add(token));
        keys.push(`${element.tagName.toLowerCase()}.${tokens.slice(0, 2).join(".")}`);
      }
      return { keys, ids, strs: [...strs.keys()] };
    };
    const previous = document.documentElement.getAttribute("data-theme");
    const previousShell = shell.getAttribute("data-theme");
    const result: Record<string, unknown> = {};
    for (const theme of ["dark", "light"]) {
      document.documentElement.setAttribute("data-theme", theme);
      shell.setAttribute("data-theme", theme);
      await sleep(120);
      result[theme] = snapshotTheme();
    }
    if (previous === null) document.documentElement.removeAttribute("data-theme"); else document.documentElement.setAttribute("data-theme", previous);
    if (previousShell === null) shell.removeAttribute("data-theme"); else shell.setAttribute("data-theme", previousShell);
    return { ...result, classes: [...classes].sort() };
  }, { props: [...PROPS], selector });
  return captured as Omit<Snapshot, "title" | "url" | "format" | "settled"> | null;
}

// Elements a user can hover, focus, or press. Forced-state snapshots restrict
// themselves to these so the files stay small.
const INTERACTIVE = ".product-shell :is(a[href], button, input, select, textarea, summary, [tabindex]:not([tabindex='-1']))";
const FORCED_STATES = ["hover", "focus-visible", "active"] as const;
/**
 * What a real user triggers: a press also hovers the control, and keyboard focus
 * matches both :focus and :focus-visible. Forcing only the named pseudo-class
 * under-reports those styles (found by e2e/interaction-fidelity.spec.ts, ADR-0433).
 */
export function forcedClassesFor(state: string): string[] {
  if (state === "active") return ["hover", "active"];
  if (state === "focus-visible") return ["focus", "focus-visible"];
  return [state];
}

// Containers that restyle when a descendant has focus (field groups, menus, disclosures).
const FOCUS_GROUPS = ".product-shell :is(label, fieldset, form, details, .panel, .account-menu-panel, .display-settings-menu, .system-status-panel)";

async function captureForcedState(page: Page, state: (typeof FORCED_STATES)[number] | "focus-within", scope = INTERACTIVE): Promise<Omit<Snapshot, "title" | "url" | "format" | "settled"> | null> {
  const client = await page.context().newCDPSession(page);
  try {
    await client.send("DOM.enable");
    await client.send("CSS.enable");
    const { root } = await client.send("DOM.getDocument", { depth: 0 });
    const { nodeIds } = await client.send("DOM.querySelectorAll", { nodeId: root.nodeId, selector: scope });
    for (const nodeId of nodeIds) await client.send("CSS.forcePseudoState", { nodeId, forcedPseudoClasses: forcedClassesFor(state) });
    const result = await capture(page, scope);
    for (const nodeId of nodeIds) await client.send("CSS.forcePseudoState", { nodeId, forcedPseudoClasses: [] });
    return result;
  } finally {
    await client.detach().catch(() => undefined);
  }
}

async function captureOpenDisclosures(page: Page): Promise<Omit<Snapshot, "title" | "url" | "format" | "settled"> | null> {
  const opened = await page.evaluate(() => {
    const closed = [...document.querySelectorAll<HTMLDetailsElement>(".product-shell details:not([open])")];
    closed.forEach((element) => { element.setAttribute("open", ""); element.setAttribute("data-style-snapshot-opened", ""); });
    return closed.length;
  });
  if (opened === 0) return null;
  const result = await capture(page);
  await page.evaluate(() => document.querySelectorAll<HTMLDetailsElement>(".product-shell details[data-style-snapshot-opened]").forEach((element) => { element.removeAttribute("open"); element.removeAttribute("data-style-snapshot-opened"); }));
  return result;
}

/** Resolve once the shell has had no DOM mutations for a quiet period (or a cap), so async loads and ticks cannot race a capture. */
async function settle(page: Page): Promise<boolean> {
  return page.evaluate(() => new Promise<boolean>((resolve) => {
    const target = document.querySelector(".product-shell") ?? document.body;
    let timer = window.setTimeout(() => finish(true), 350);
    const cap = window.setTimeout(() => finish(false), 4000);
    const observer = new MutationObserver(() => {
      window.clearTimeout(timer);
      timer = window.setTimeout(() => finish(true), 350);
    });
    observer.observe(target, { childList: true, subtree: true, attributes: true, characterData: true });
    function finish(quiet: boolean) {
      observer.disconnect();
      window.clearTimeout(timer);
      window.clearTimeout(cap);
      resolve(quiet);
    }
  }));
}

const counters = new Map<string, number>();

/** Record a snapshot at a labelled state inside a journey (no-op unless enabled). */
export async function captureStyleSnapshot(page: Page, title: string, label: string): Promise<void> {
  const directory = process.env.STYLE_SNAPSHOT_DIR;
  if (!directory || page.isClosed()) return;
  const settled = await settle(page);
  const captured = await capture(page);
  if (!captured) return;
  const key = `${title} @ ${label}`;
  const count = (counters.get(key) ?? 0) + 1;
  counters.set(key, count);
  await mkdir(directory, { recursive: true });
  const snapshot: Snapshot = { format: SNAPSHOT_FORMAT, settled, title: key, url: page.url(), ...captured };
  const base = fileName(count > 1 ? `${key} ${count}` : key);
  await writeFile(join(directory, base), JSON.stringify(snapshot));
  if (!process.env.STYLE_SNAPSHOT_NO_STATES) {
    for (const state of FORCED_STATES) {
      const forced = await captureForcedState(page, state);
      if (forced) await writeFile(join(directory, base.replace(/\.json$/, `.${state}.json`)), JSON.stringify({ format: SNAPSHOT_FORMAT, settled, title: `${key} [${state}]`, url: page.url(), ...forced } satisfies Snapshot));
    }
    const within = await captureForcedState(page, "focus-within", FOCUS_GROUPS);
    if (within) await writeFile(join(directory, base.replace(/\.json$/, ".focus-within.json")), JSON.stringify({ format: SNAPSHOT_FORMAT, settled, title: `${key} [focus-within]`, url: page.url(), ...within } satisfies Snapshot));
    const open = await captureOpenDisclosures(page);
    if (open) await writeFile(join(directory, base.replace(/\.json$/, ".open.json")), JSON.stringify({ format: SNAPSHOT_FORMAT, settled, title: `${key} [open]`, url: page.url(), ...open } satisfies Snapshot));
  }
  if (process.env.STYLE_SNAPSHOT_SCREENSHOTS) {
    // A full-page image of the same state, in both themes, for human review.
    await mkdir(join(directory, "images"), { recursive: true });
    const original = await page.evaluate(() => ({
      root: document.documentElement.getAttribute("data-theme"),
      shell: document.querySelector(".product-shell")?.getAttribute("data-theme") ?? null,
    }));
    for (const theme of ["dark", "light"]) {
      await page.evaluate((value: string) => {
        document.documentElement.setAttribute("data-theme", value);
        document.querySelector(".product-shell")?.setAttribute("data-theme", value);
      }, theme);
      await page.screenshot({ path: join(directory, "images", base.replace(/\.json$/, `.${theme}.png`)), fullPage: true });
    }
    await page.evaluate((previous: { root: string | null; shell: string | null }) => {
      const shell = document.querySelector(".product-shell");
      if (previous.root === null) document.documentElement.removeAttribute("data-theme"); else document.documentElement.setAttribute("data-theme", previous.root);
      if (previous.shell === null) shell?.removeAttribute("data-theme"); else shell?.setAttribute("data-theme", previous.shell);
    }, original);
  }
}

export function attachStyleSnapshots(testApi: TestType<any, any>): void {
  if (!process.env.STYLE_SNAPSHOT_DIR) return;
  // A journey with many checkpoints (each recording interactive variants in two
  // themes) can exceed the 30 s default, especially under parallel load.
  testApi.beforeEach(async ({}, testInfo) => {
    testInfo.setTimeout(Math.max(testInfo.timeout, 120_000));
  });
  testApi.afterEach(async ({ page }, testInfo) => {
    if (testInfo.status !== "passed") return;
    await captureStyleSnapshot(page, testInfo.title, "end");
  });
}
