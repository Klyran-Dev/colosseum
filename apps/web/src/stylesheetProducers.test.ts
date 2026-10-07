// @vitest-environment node

import { readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { describe, expect, it } from "vitest";

const sourceRoot = resolve(process.cwd(), "src");
const stylesheet = readFileSync(join(sourceRoot, "styles.css"), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");

function productionSource(): string {
  return readdirSync(sourceRoot)
    .filter((name) => /\.(ts|tsx)$/.test(name) && !/\.test\.(ts|tsx)$/.test(name))
    .map((name) => readFileSync(join(sourceRoot, name), "utf8"))
    .join("\n");
}

// A class that is only built dynamically (`risk-${value}`) has no verbatim
// occurrence, so a prefix followed by a template expression is a producer.
function hasProducer(token: string, source: string): boolean {
  if (source.includes(token)) return true;
  const parts = token.split("-");
  for (let end = 1; end < parts.length; end++) {
    if (source.includes(`${parts.slice(0, end).join("-")}-\${`)) return true;
  }
  return false;
}

describe("stylesheet class producers (ADR-0428)", () => {
  it("has a producer in production source for every class the stylesheet styles", () => {
    const source = productionSource();
    const tokens = new Set([...stylesheet.matchAll(/\.([a-zA-Z][\w-]*)/g)].map((match) => match[1]));
    const orphaned = [...tokens].filter((token) => !hasProducer(token, source)).sort();
    expect(orphaned, "remove CSS rules for classes nothing renders").toEqual([]);
  });
});

/**
 * Classes that no real-browser journey or captured fixture renders yet (see
 * e2e/unverified-surfaces.json), so a stylesheet change to
 * them cannot be proven neutral by scripts/diff_style_snapshots.mjs. Rules that
 * target only these classes may keep hard-coded rem/px type and corner sizes.
 * Remove a class from this list as soon as a journey renders it and its rules
 * are on tokens; adding to it needs a reason in the ADR (ADR-0428).
 */
const UNVERIFIED_SURFACES = new Set<string>(
  JSON.parse(readFileSync(resolve(process.cwd(), "e2e/unverified-surfaces.json"), "utf8")),
);

describe("type scale and shape tokens everywhere verifiable (ADR-0428)", () => {
  it("keeps hard-coded font sizes and corner radii only on unverified surfaces", () => {
    const literal = /font-size:\s*[\d.]+rem|border(?:-[a-z]+)*-radius:\s*[^;}]*(?:[\d.]+rem|[\d.]+px)/;
    const offenders: string[] = [];
    for (const match of stylesheet.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
      const [, selector, body] = match;
      if (!literal.test(body) || selector.trim().startsWith("@")) continue;
      const classes = [...selector.matchAll(/\.([a-zA-Z][\w-]*)/g)].map((entry) => entry[1]);
      if (classes.some((token) => UNVERIFIED_SURFACES.has(token))) continue;
      offenders.push(selector.trim().replace(/\s+/g, " ").slice(0, 90));
    }
    expect([...new Set(offenders)], "move these rules onto type-scale and shape tokens").toEqual([]);
  });

  it("does not list a class as unverified once nothing styles it", () => {
    const styled = new Set([...stylesheet.matchAll(/\.([a-zA-Z][\w-]*)/g)].map((entry) => entry[1]));
    expect([...UNVERIFIED_SURFACES].filter((token) => !styled.has(token))).toEqual([]);
  });
});
