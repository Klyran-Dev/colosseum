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

interface Snapshot {
  title: string;
  url: string;
  classes: string[];
  dark: SnapshotTheme;
  light: SnapshotTheme;
}

function fileName(title: string): string {
  return `${title.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "").slice(0, 200)}.json`;
}

async function capture(page: Page, selector = ".product-shell, .product-shell *"): Promise<Omit<Snapshot, "title" | "url"> | null> {
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
  return captured as Omit<Snapshot, "title" | "url"> | null;
}

// Elements a user can hover, focus, or press. Forced-state snapshots restrict
// themselves to these so the files stay small.
const INTERACTIVE = ".product-shell :is(a[href], button, input, select, textarea, summary, [tabindex]:not([tabindex='-1']))";
const FORCED_STATES = ["hover", "focus-visible", "active"] as const;

async function captureForcedState(page: Page, state: (typeof FORCED_STATES)[number]): Promise<Omit<Snapshot, "title" | "url"> | null> {
  const client = await page.context().newCDPSession(page);
  try {
    await client.send("DOM.enable");
    await client.send("CSS.enable");
    const { root } = await client.send("DOM.getDocument", { depth: 0 });
    const { nodeIds } = await client.send("DOM.querySelectorAll", { nodeId: root.nodeId, selector: INTERACTIVE });
    for (const nodeId of nodeIds) await client.send("CSS.forcePseudoState", { nodeId, forcedPseudoClasses: [state] });
    const result = await capture(page, INTERACTIVE);
    for (const nodeId of nodeIds) await client.send("CSS.forcePseudoState", { nodeId, forcedPseudoClasses: [] });
    return result;
  } finally {
    await client.detach().catch(() => undefined);
  }
}

async function captureOpenDisclosures(page: Page): Promise<Omit<Snapshot, "title" | "url"> | null> {
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

const counters = new Map<string, number>();

/** Record a snapshot at a labelled state inside a journey (no-op unless enabled). */
export async function captureStyleSnapshot(page: Page, title: string, label: string): Promise<void> {
  const directory = process.env.STYLE_SNAPSHOT_DIR;
  if (!directory || page.isClosed()) return;
  const captured = await capture(page);
  if (!captured) return;
  const key = `${title} @ ${label}`;
  const count = (counters.get(key) ?? 0) + 1;
  counters.set(key, count);
  await mkdir(directory, { recursive: true });
  const snapshot: Snapshot = { title: key, url: page.url(), ...captured };
  const base = fileName(count > 1 ? `${key} ${count}` : key);
  await writeFile(join(directory, base), JSON.stringify(snapshot));
  if (!process.env.STYLE_SNAPSHOT_NO_STATES) {
    for (const state of FORCED_STATES) {
      const forced = await captureForcedState(page, state);
      if (forced) await writeFile(join(directory, base.replace(/\.json$/, `.${state}.json`)), JSON.stringify({ title: `${key} [${state}]`, url: page.url(), ...forced } satisfies Snapshot));
    }
    const open = await captureOpenDisclosures(page);
    if (open) await writeFile(join(directory, base.replace(/\.json$/, ".open.json")), JSON.stringify({ title: `${key} [open]`, url: page.url(), ...open } satisfies Snapshot));
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
  testApi.afterEach(async ({ page }, testInfo) => {
    if (testInfo.status !== "passed") return;
    await captureStyleSnapshot(page, testInfo.title, "end");
  });
}
