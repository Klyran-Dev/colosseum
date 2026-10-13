import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const stylesheet = readFileSync(resolve(process.cwd(), "src/admin/admin.css"), "utf8");

describe("admin typography contract", () => {
  it("defines an explicit readable base scale and control size", () => {
    expect(stylesheet).toMatch(/body\s*\{[^}]*font-size:\s*1rem;[^}]*line-height:\s*1\.5;/u);
    expect(stylesheet).toMatch(/input,\s*textarea,\s*select\s*\{[^}]*font-size:\s*1rem;[^}]*line-height:\s*1\.5;/u);
  });

  it("ships the selected display font and uses it as the admin type stack", () => {
    expect(stylesheet).toContain('@import "@fontsource-variable/plus-jakarta-sans";');
    expect(stylesheet).toMatch(/--admin-font-body:\s*"Plus Jakarta Sans",\s*ui-sans-serif,\s*system-ui/u);
    expect(stylesheet).toContain("font-family: var(--admin-font-body);");
  });

  it("keeps the sign-in hierarchy compact before the desktop breakpoint", () => {
    expect(stylesheet).toContain(".admin-auth-title");
    expect(stylesheet).toMatch(/\.admin-auth-title\s*\{[^}]*font-size:\s*2rem;/u);
    expect(stylesheet).toMatch(/@media\s*\(min-width:\s*64rem\)[\s\S]*\.admin-auth-title\s*\{[^}]*font-size:\s*2\.75rem;/u);
    expect(stylesheet).toContain(".admin-auth-shell");
    expect(stylesheet).toMatch(/\.admin-auth-shell\s*\{[^}]*min-height:\s*0;/u);
  });
});
