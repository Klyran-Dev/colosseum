import axe from "axe-core";
import type { Page } from "@playwright/test";

/** The accessibility standard this product commits to (AGENTS.md): WCAG 2.2 AA. */
export const AXE_TAGS = ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"] as const;
export const AUDIT_THEMES = ["dark", "light"] as const;

export interface AxeViolation {
  theme: string;
  id: string;
  impact: string | null;
  help: string;
  nodes: Array<{ target: string[]; html: string }>;
}

/**
 * Run axe-core in the real browser against the current page state in every
 * supported theme, then restore the theme the page had. Both themes matter:
 * contrast is a property of each palette, and the light scheme is a first-class
 * product surface.
 */
export interface AxeAuditOptions {
  tags?: readonly string[];
  themes?: readonly string[];
  /**
   * Rules to switch off. Only for states where the rule cannot be evaluated
   * truthfully; each use must say why (see the forced-colors journey).
   */
  disabledRules?: readonly string[];
}

export async function axeViolationsInAllThemes(page: Page, options: AxeAuditOptions = {}): Promise<AxeViolation[]> {
  const { tags = AXE_TAGS, themes = AUDIT_THEMES, disabledRules = [] } = options;
  const collected: AxeViolation[] = [];
  // The production CSP intentionally allows only same-origin scripts. Loading
  // axe through a same-origin Playwright route exercises that policy instead
  // of weakening it with an inline script or eval-based test escape hatch.
  const axeUrl = new URL("/__e2e/axe-core.js", page.url()).toString();
  const axeRoute = async (route: Parameters<NonNullable<Parameters<Page["route"]>[1]>>[0]) => {
    await route.fulfill({ status: 200, contentType: "application/javascript", body: axe.source });
  };
  await page.route(axeUrl, axeRoute);
  const original = await page.evaluate(() => ({
    root: document.documentElement.getAttribute("data-theme"),
    shell: document.querySelector(".product-shell")?.getAttribute("data-theme") ?? null,
  }));
  try {
    for (const theme of themes) {
      await page.evaluate((value: string) => {
        document.documentElement.setAttribute("data-theme", value);
        document.querySelector(".product-shell")?.setAttribute("data-theme", value);
      }, theme);
      // Theme changes animate semantic surfaces and controls. Wait past the
      // longest documented expressive transition before sampling contrast;
      // getAnimations() also includes the shell's intentionally persistent
      // motion definitions and is therefore not a reliable completion signal.
      await page.waitForTimeout(350);
      if (!(await page.evaluate(() => "axe" in window))) {
        await page.addScriptTag({ url: axeUrl });
      }
      const violations = await page.evaluate(async ({ ruleTags, off }: { ruleTags: string[]; off: string[] }) => {
        const browserAxe = (window as unknown as {
          axe?: {
            run: (context: Document, options: { runOnly: { type: "tag"; values: string[] }; rules: Record<string, { enabled: boolean }> }) => Promise<{
              violations: Array<{ id: string; impact: string | null; help: string; nodes: Array<{ target: string[]; html: string }> }>;
            }>;
          };
        }).axe;
        if (!browserAxe) throw new Error("axe-core did not initialize in the real browser");
        const result = await browserAxe.run(document, { runOnly: { type: "tag", values: ruleTags }, rules: Object.fromEntries(off.map((rule) => [rule, { enabled: false }])) });
        return result.violations.map(({ id, impact, help, nodes }) => ({
          id,
          impact,
          help,
          nodes: nodes.map(({ target, html }) => ({ target, html })),
        }));
      }, { ruleTags: [...tags], off: [...disabledRules] });
      collected.push(...violations.map((violation) => ({ theme, ...violation })));
    }
  } finally {
    await page.unroute(axeUrl, axeRoute);
    await page.evaluate((previous: { root: string | null; shell: string | null }) => {
      const shell = document.querySelector(".product-shell");
      if (previous.root === null) document.documentElement.removeAttribute("data-theme"); else document.documentElement.setAttribute("data-theme", previous.root);
      if (previous.shell === null) shell?.removeAttribute("data-theme"); else shell?.setAttribute("data-theme", previous.shell);
    }, original);
  }
  return collected;
}
