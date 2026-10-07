// @vitest-environment node

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const config = readFileSync(resolve(process.cwd(), "vite.config.ts"), "utf8");
const stylesheet = readFileSync(resolve(process.cwd(), "src/styles.css"), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");

describe("browser baseline for the stylesheet (ADR-0436)", () => {
  it("states the CSS target the build must preserve", () => {
    expect(config).toContain('cssTarget: ["chrome113", "edge113", "firefox113", "safari17.2"]');
  });

  it("uses only features inside that baseline", () => {
    // Each of these shipped in the target browsers before the baseline versions.
    expect(stylesheet).toContain("color-mix(");
    expect(stylesheet).toContain("linear(");
    expect(stylesheet).toContain("@container ");
    // Features newer than the baseline would break it.
    expect(stylesheet).not.toMatch(/(^|[\s{;])@scope\b|anchor-name:|position-anchor:|@starting-style|field-sizing|text-wrap:\s*balance\s*;?\s*\/\*|interpolate-size|corner-shape|::details-content/);
    expect(stylesheet).not.toMatch(/\{[^{}]*&\s/);
  });
});
