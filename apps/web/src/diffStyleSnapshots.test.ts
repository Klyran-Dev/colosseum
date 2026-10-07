// @vitest-environment node

import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { afterAll, describe, expect, it } from "vitest";

const script = resolve(process.cwd(), "../../scripts/diff_style_snapshots.mjs");
const root = mkdtempSync(join(tmpdir(), "style-diff-"));
afterAll(() => rmSync(root, { recursive: true, force: true }));

// 33 properties in the order the tool expects, then the "WxH" size.
const PROPS = 33;
function row(overrides: Record<number, string> = {}, size = "100x20"): string {
  return [...Array.from({ length: PROPS }, (_, index) => overrides[index] ?? `v${index}`), size].join("|");
}

function writeSnapshot(directory: string, name: string, rows: string[], classes = ["panel"], extra: Record<string, unknown> = {}): void {
  mkdirSync(directory, { recursive: true });
  const theme = { keys: rows.map((_, index) => `div.item${index}`), ids: rows.map((_, index) => index), strs: rows };
  writeFileSync(join(directory, name), JSON.stringify({ format: 3, settled: true, title: name, url: "/", classes, dark: theme, light: theme, ...extra }));
}

function run(...args: string[]) {
  const result = spawnSync("node", [script, ...args], { encoding: "utf8" });
  return { status: result.status, output: `${result.stdout}${result.stderr}` };
}

describe("style snapshot diff tool (ADR-0428)", () => {
  it("reports zero differences for identical snapshots", () => {
    writeSnapshot(join(root, "a1"), "state.json", [row(), row({ 13: "16px" })]);
    writeSnapshot(join(root, "b1"), "state.json", [row(), row({ 13: "16px" })]);
    const result = run(join(root, "a1"), join(root, "b1"));
    expect(result.status).toBe(0);
    expect(result.output).toContain("0 element-diffs");
  });

  it("names the property that changed and fails", () => {
    writeSnapshot(join(root, "a2"), "state.json", [row({ 13: "11px" })]);
    writeSnapshot(join(root, "b2"), "state.json", [row({ 13: "12px" })]);
    const result = run(join(root, "a2"), join(root, "b2"));
    expect(result.status).toBe(1);
    expect(result.output).toContain("fontSize 11px->12px");
    const props = run(join(root, "a2"), join(root, "b2"), "--props", "--expect", "borderTopLeftRadius");
    expect(props.status).toBe(1);
    expect(props.output).toContain("UNEXPECTED");
  });

  it("accepts a change confined to the expected properties", () => {
    writeSnapshot(join(root, "a3"), "state.json", [row({ 13: "11px" })]);
    writeSnapshot(join(root, "b3"), "state.json", [row({ 13: "12px" })]);
    expect(run(join(root, "a3"), join(root, "b3"), "--props", "--expect", "fontSize").status).toBe(0);
  });

  it("treats a different element count as a structural issue and never exempts any state", () => {
    writeSnapshot(join(root, "a4"), "expired-quote-checkout-state.json", [row(), row()]);
    writeSnapshot(join(root, "b4"), "expired-quote-checkout-state.json", [row()]);
    const result = run(join(root, "a4"), join(root, "b4"));
    expect(result.status).toBe(1);
    expect(result.output).toContain("element count 2 -> 1");
  });

  it("lists styled classes that no captured state renders", () => {
    writeSnapshot(join(root, "c1"), "state.json", [row()], ["panel"]);
    const sheet = join(root, "styles.css");
    writeFileSync(sheet, ".panel { color: red; }\n.orphaned-card { color: red; }\n");
    const result = run("--coverage", join(root, "c1"), sheet);
    expect(result.output).toContain("never rendered: 1");
    expect(result.output).toContain("orphaned-card");
  });

  it("refuses to compare snapshots taken with a different capture format", () => {
    writeSnapshot(join(root, "a5"), "state.json", [row()], ["panel"], { format: 2 });
    writeSnapshot(join(root, "b5"), "state.json", [row()]);
    const result = run(join(root, "a5"), join(root, "b5"));
    expect(result.status).toBe(2);
    expect(result.output).toMatch(/not format 3/);
    expect(result.output).toMatch(/Re-take this baseline/);
  });

  it("reports a capture that never settled instead of silently comparing it", () => {
    writeSnapshot(join(root, "a6"), "state.json", [row()], ["panel"], { settled: false });
    writeSnapshot(join(root, "b6"), "state.json", [row()]);
    const result = run(join(root, "a6"), join(root, "b6"));
    expect(result.status).toBe(1);
    expect(result.output).toContain("unsettled capture");
  });
});
