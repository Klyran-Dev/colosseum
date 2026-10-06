// @vitest-environment node

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
// @ts-expect-error build-time ESM helper without type declarations
import { generatePalette, SEED } from "../scripts/material-palette.mjs";
// @ts-expect-error build-time ESM helper without type declarations
import { springTokens } from "../scripts/material-motion.mjs";

const stylesheet = readFileSync(resolve(process.cwd(), "src/styles.css"), "utf8");
const palette = generatePalette() as Record<"light" | "dark", Record<string, string>>;
const springs = springTokens() as Record<string, { duration: string; easing: string }>;

function block(pattern: RegExp): string {
  const found = stylesheet.match(pattern)?.[1];
  if (!found) throw new Error(`missing block ${pattern}`);
  return found;
}

const darkBlock = () => block(/^:root\s*\{([\s\S]*?)\n\}/m);
const lightBlock = () => block(/^:root\[data-theme="light"\]\s*\{([\s\S]*?)\n\}/m);
const systemLightBlock = () => block(/:root\[data-theme="system"\], \.product-shell\[data-theme="system"\] \{([\s\S]*?)\n  \}/m);

function tokens(source: string): Map<string, string> {
  return new Map([...source.matchAll(/--md-sys-color-([\w-]+):\s*(#[0-9A-Fa-f]{6})\s*;/g)].map((m) => [m[1], m[2].toUpperCase()]));
}

describe("Material 3 palette generated from the Material baseline seed (ADR-0422)", () => {
  it("uses the Material baseline seed and declares provenance", () => {
    expect(SEED).toBe("#6750A4");
    expect(stylesheet).toContain("Material 3 Expressive theme family: Material Baseline / seed #6750A4 / TonalSpot / role matrix v3");
  });

  for (const [scheme, source] of [["dark", darkBlock], ["light", lightBlock], ["light (system)", systemLightBlock]] as const) {
    it(`declares every generated ${scheme} role exactly`, () => {
      const declared = tokens(source());
      const expected = palette[scheme === "dark" ? "dark" : "light"];
      for (const [role, hex] of Object.entries(expected)) {
        expect(declared.get(role), `${scheme} ${role}`).toBe(hex);
      }
      expect(declared.size).toBe(Object.keys(expected).length);
    });
  }

  it("contains no green or teal brand hues in the primary family", () => {
    for (const scheme of ["light", "dark"] as const) {
      const hex = palette[scheme].primary;
      const [r, g, b] = [1, 3, 5].map((i) => Number.parseInt(hex.slice(i, i + 2), 16));
      expect(g, `${scheme} primary must not be green-dominant`).toBeLessThanOrEqual(Math.max(r, b));
    }
  });
});

describe("Material 3 Expressive shape, motion, type and state tokens (ADR-0422)", () => {
  const root = () => darkBlock();

  it("declares the complete corner scale including Expressive additions", () => {
    const expected: Record<string, string> = {
      none: "0", "extra-small": ".25rem", small: ".5rem", medium: ".75rem", large: "1rem",
      "large-increased": "1.25rem", "extra-large": "1.75rem", "extra-large-increased": "2rem",
      "extra-extra-large": "3rem", full: "999rem",
    };
    for (const [name, value] of Object.entries(expected)) {
      expect(root()).toContain(`--md-sys-shape-corner-${name}: ${value};`);
    }
  });

  it("declares all six spring tokens from the generated physics", () => {
    for (const [name, spring] of Object.entries(springs)) {
      expect(root()).toContain(`--md-sys-motion-spring-${name}-duration: ${spring.duration};`);
      expect(root()).toContain(`--md-sys-motion-spring-${name}-easing: ${spring.easing};`);
    }
  });

  it("declares the type scale with emphasized variants", () => {
    for (const role of ["display", "headline", "title", "body", "label"]) {
      for (const size of ["large", "medium", "small"]) {
        expect(root()).toContain(`--md-sys-typescale-${role}-${size}-size:`);
        expect(root()).toContain(`--md-sys-typescale-${role}-${size}-line-height:`);
        expect(root()).toContain(`--md-sys-typescale-${role}-${size}-weight:`);
        expect(root()).toContain(`--md-sys-typescale-${role}-${size}-emphasized-weight: 700;`);
      }
    }
    expect(root()).toContain("--md-sys-typescale-body-large-size: 1rem;");
    expect(root()).toContain("--md-sys-typescale-display-large-size: 3.5625rem;");
  });

  it("declares the state-layer opacities", () => {
    expect(root()).toContain("--md-sys-state-hover-opacity: .08;");
    expect(root()).toContain("--md-sys-state-focus-opacity: .1;");
    expect(root()).toContain("--md-sys-state-pressed-opacity: .1;");
    expect(root()).toContain("--md-sys-state-dragged-opacity: .16;");
  });
});
