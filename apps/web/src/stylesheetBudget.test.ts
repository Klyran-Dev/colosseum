// @vitest-environment node

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { gzipSync } from "node:zlib";
import { describe, expect, it } from "vitest";

/**
 * Performance budget for the one stylesheet every route downloads and parses
 * (ADR-0436). Measured when set: 123 KB source, about 14 KB gzipped in the build.
 * The ceilings leave roughly 25% headroom; raise them deliberately in an ADR, do
 * not let the stylesheet grow unnoticed.
 */
const MAX_SOURCE_BYTES = 155_000;
const MAX_GZIP_BYTES = 26_000;
const MAX_RULES = 950;

const source = readFileSync(resolve(process.cwd(), "src/styles.css"), "utf8");

describe("stylesheet performance budget (ADR-0436)", () => {
  it("stays within the source and compressed size ceilings", () => {
    expect(Buffer.byteLength(source), "source bytes").toBeLessThanOrEqual(MAX_SOURCE_BYTES);
    expect(gzipSync(source).byteLength, "gzip bytes of the source").toBeLessThanOrEqual(MAX_GZIP_BYTES);
  });

  it("keeps the rule count bounded", () => {
    const withoutComments = source.replace(/\/\*[\s\S]*?\*\//g, "");
    const rules = (withoutComments.match(/\{/g) ?? []).length;
    expect(rules, "rule blocks").toBeLessThanOrEqual(MAX_RULES);
  });

  it("does not import fonts or other external resources from CSS", () => {
    expect(source).not.toMatch(/@import\s/);
    expect(source).not.toMatch(/url\(\s*["']?https?:/i);
  });
});
