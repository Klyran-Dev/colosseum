// @vitest-environment node

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { gzipSync } from "node:zlib";
import { describe, expect, it } from "vitest";

/**
 * Performance budget for the one stylesheet every route downloads and parses
 * (ADR-0436). The licensed Demo 3 adapter adds the shell/auth composition while
 * keeping the compressed artifact below the original 26 KB ceiling. The source
 * and rule ceilings were deliberately revised in ADR-0490 and must not grow
 * unnoticed.
 */
const MAX_SOURCE_BYTES = 170_000;
const MAX_GZIP_BYTES = 26_000;
const MAX_RULES = 1_050;

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
