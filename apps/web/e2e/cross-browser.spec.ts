import { expect, test } from "@playwright/test";
import { axeViolationsInAllThemes } from "./axeAudit";

/**
 * The supported-browser baseline (Chrome/Edge 113+, Firefox 113+, Safari 17.2+) is
 * stated in the build and tested against the stylesheet; this suite runs the product
 * in the Firefox and WebKit engines (ADR-0439). It is skipped under Chromium, where
 * the rest of the gate already runs.
 */
test.skip(({ browserName }) => browserName === "chromium", "cross-engine suite runs under Firefox and WebKit");

const ROUTES = ["/operations/tax-quotes", "/rules/tax-rules", "/rules/access-approvals", "/money/treasury"];

test("the engine supports every modern CSS feature the stylesheet uses", async ({ page }) => {
  await page.goto("/");
  const support = await page.evaluate(() => ({
    colorMix: CSS.supports("color", "color-mix(in srgb, red 50%, blue)"),
    linearEasing: CSS.supports("transition-timing-function", "linear(0, 0.5, 1)"),
    containerQueries: CSS.supports("container-type", "inline-size"),
    isSelector: CSS.supports("selector(:is(a, b))"),
    focusVisible: CSS.supports("selector(:focus-visible)"),
    insetInline: CSS.supports("inset-inline", "0"),
    hoverMedia: window.matchMedia("(hover: hover)").matches,
  }));
  expect(support).toEqual({ colorMix: true, linearEasing: true, containerQueries: true, isSelector: true, focusVisible: true, insetInline: true, hoverMedia: true });
});

test("the app renders with its design tokens and without script or console errors", async ({ page }) => {
  const problems: string[] = [];
  page.on("pageerror", (error) => problems.push(`pageerror: ${error.message}`));
  page.on("console", (message) => { if (message.type() === "error") problems.push(`console: ${message.text()}`); });
  await page.goto("/operations/tax-quotes");
  await page.waitForLoadState("networkidle");
  await expect(page.getByRole("heading", { level: 1, name: "Tax quotes" })).toBeVisible();
  for (const theme of ["dark", "light"]) {
    await page.evaluate((value: string) => {
      document.documentElement.setAttribute("data-theme", value);
      document.querySelector(".product-shell")?.setAttribute("data-theme", value);
    }, theme);
    const tokens = await page.evaluate(() => {
      const root = getComputedStyle(document.documentElement);
      const active = document.querySelector<HTMLElement>(".nav-item-active");
      const probe = document.createElement("span");
      probe.style.color = "var(--md-sys-color-primary-container)";
      document.body.appendChild(probe);
      const expected = getComputedStyle(probe).color;
      probe.remove();
      return {
        primary: root.getPropertyValue("--md-sys-color-primary").trim(),
        activeBackground: active ? getComputedStyle(active).backgroundColor : null,
        expectedBackground: expected,
        panelRadius: getComputedStyle(document.querySelector(".panel")!).borderTopLeftRadius,
      };
    });
    expect(tokens.primary, `${theme}: palette token resolves`).toMatch(/^#[0-9A-Fa-f]{6}$/);
    expect(tokens.activeBackground, `${theme}: active navigation uses the primary-container role`).toBe(tokens.expectedBackground);
    expect(tokens.panelRadius, `${theme}: panels use the extra-large corner token`).toBe("28px");
  }
  expect(problems).toEqual([]);
});

test.describe("motion", () => {
  // The shared config emulates reduced motion, which correctly turns transitions off.
  test.use({ reducedMotion: "no-preference" });

  test("controls transition with the spring tokens and paint a hover state layer", async ({ page }) => {
  await page.goto("/");
  const nav = page.getByRole("button", { name: "Tax simulator", exact: true });
  const timing = await nav.evaluate((element) => {
    const style = getComputedStyle(element);
    return { duration: style.transitionDuration, easing: style.transitionTimingFunction };
  });
  expect(timing.duration).not.toBe("0s");
  expect(timing.easing).toMatch(/linear\(|cubic-bezier|ease/);
  await nav.hover();
  await page.waitForFunction(() => document.getAnimations().length === 0, undefined, { timeout: 3_000 });
  const hovered = await nav.evaluate((element) => getComputedStyle(element).backgroundColor);
  await page.mouse.move(2, 2);
  await page.waitForFunction(() => document.getAnimations().length === 0, undefined, { timeout: 3_000 });
  const rested = await nav.evaluate((element) => getComputedStyle(element).backgroundColor);
  expect(hovered, "hover paints a state layer").not.toBe(rested);
  });

  test("reduced motion removes control transitions in this engine too", async ({ browser }) => {
    const context = await browser.newContext({ reducedMotion: "reduce" });
    const page = await context.newPage();
    await page.goto("/");
    const durations = await page.evaluate(() => [...document.querySelectorAll(".product-shell button, .product-shell summary, .product-shell input")]
      .map((element) => getComputedStyle(element).transitionDuration)
      .filter((value) => value !== "0s"));
    expect(durations).toEqual([]);
    await context.close();
  });
});

for (const route of ROUTES) {
  test(`${route} meets WCAG 2.2 AA in both themes and fits 320px`, async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 900 });
    await page.goto(route);
    await page.waitForLoadState("networkidle");
    const violations = await axeViolationsInAllThemes(page);
    expect(violations, `${route}: WCAG 2.2 AA violations`).toEqual([]);
    await page.setViewportSize({ width: 320, height: 800 });
    await page.evaluate(() => document.querySelectorAll(".product-shell details").forEach((element) => element.setAttribute("open", "")));
    const width = await page.evaluate(() => ({ scroll: document.documentElement.scrollWidth, viewport: document.documentElement.clientWidth }));
    expect(width.scroll, `${route}: no horizontal scroll at 320px`).toBeLessThanOrEqual(width.viewport + 1);
  });
}
