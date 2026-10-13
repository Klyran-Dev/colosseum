// @vitest-environment node

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const entrypoint = readFileSync(resolve(process.cwd(), "src/main.tsx"), "utf8");

describe("admin application entrypoint", () => {
  it("mounts the new admin application and does not import the retired UI tree", () => {
    expect(entrypoint).toContain('import AdminApp from "./admin/AdminApp";');
    expect(entrypoint).toContain('import "./admin/admin.css";');
    expect(entrypoint).not.toContain('from "./App"');
    expect(entrypoint).not.toContain('from "./ProductShell"');
    expect(entrypoint).not.toContain('import "./styles.css"');
  });
});
