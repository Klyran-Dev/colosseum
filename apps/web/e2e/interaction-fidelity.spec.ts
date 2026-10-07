import { expect, test, type Locator, type Page } from "@playwright/test";
import { forcedClassesFor } from "./styleSnapshot";

/**
 * Proves that the forced pseudo-state snapshots (e2e/styleSnapshot.ts) match what
 * a real user gets (ADR-0433): real mouse hover and press and real keyboard focus,
 * with transitions running, must settle on exactly the forced-state style. It also
 * checks the spring motion tokens are what the controls actually transition with,
 * and that reduced motion switches them off.
 */
const STYLE_PROPS = [
  "color", "backgroundColor", "borderTopColor", "borderRightColor", "borderBottomColor", "borderLeftColor",
  "borderTopLeftRadius", "boxShadow", "outlineStyle", "outlineWidth", "outlineColor", "textDecorationLine",
] as const;

async function computed(locator: Locator): Promise<Record<string, string>> {
  return locator.evaluate((element, props) => {
    const style = getComputedStyle(element);
    return Object.fromEntries(props.map((prop) => [prop, (style as unknown as Record<string, string>)[prop]]));
  }, [...STYLE_PROPS]);
}

async function settleTransitions(page: Page): Promise<void> {
  await page.waitForFunction(() => document.getAnimations().length === 0, undefined, { timeout: 3_000 });
}

async function forced(page: Page, locator: Locator, state: "hover" | "active" | "focus-visible"): Promise<Record<string, string>> {
  const client = await page.context().newCDPSession(page);
  try {
    await client.send("DOM.enable");
    await client.send("CSS.enable");
    await locator.evaluate((element) => element.setAttribute("data-fidelity-target", ""));
    const { root } = await client.send("DOM.getDocument", { depth: 0 });
    const { nodeId } = await client.send("DOM.querySelector", { nodeId: root.nodeId, selector: "[data-fidelity-target]" });
    await client.send("CSS.forcePseudoState", { nodeId, forcedPseudoClasses: forcedClassesFor(state) });
    await settleTransitions(page);
    const result = await computed(locator);
    await client.send("CSS.forcePseudoState", { nodeId, forcedPseudoClasses: [] });
    await locator.evaluate((element) => element.removeAttribute("data-fidelity-target"));
    return result;
  } finally {
    await client.detach().catch(() => undefined);
  }
}

const CONTROLS: Array<{ name: string; locator: (page: Page) => Locator }> = [
  { name: "navigation item", locator: (page) => page.getByRole("button", { name: "Tax simulator", exact: true }) },
  { name: "display settings chip", locator: (page) => page.locator(".display-settings > summary") },
  { name: "system status chip", locator: (page) => page.locator(".system-status > summary") },
  { name: "account trigger", locator: (page) => page.locator(".account-menu-trigger") },
];

test.describe("forced pseudo-states match real interaction", () => {
  // Transitions must actually run for the comparison to mean anything.
  test.use({ reducedMotion: "no-preference", viewport: { width: 1280, height: 900 } });

  for (const control of CONTROLS) {
    test(`${control.name}: real hover, press, and keyboard focus equal the forced state`, async ({ page }) => {
      await page.goto("/");
      await page.waitForLoadState("networkidle");
      const target = control.locator(page);
      await expect(target).toBeVisible();
      await page.mouse.move(2, 2);
      await settleTransitions(page);
      const resting = await computed(target);

      await target.hover();
      await settleTransitions(page);
      const realHover = await computed(target);
      await page.mouse.move(2, 2);
      await settleTransitions(page);
      expect(await computed(target), "leaving the control restores its resting style").toEqual(resting);
      expect(realHover, "real hover equals forced :hover").toEqual(await forced(page, target, "hover"));

      const box = await target.boundingBox();
      expect(box).not.toBeNull();
      await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2);
      await page.mouse.down();
      await settleTransitions(page);
      const realActive = await computed(target);
      // Release away from the control so the press does not become a click
      // (which would navigate or toggle a disclosure and change later states).
      await page.mouse.move(2, 2);
      await page.mouse.up();
      await settleTransitions(page);
      expect(realActive, "real press equals forced :active").toEqual(await forced(page, target, "active"));

      await page.mouse.move(2, 2);
      await target.focus();
      await page.keyboard.press("Shift+Tab");
      await page.keyboard.press("Tab");
      await settleTransitions(page);
      expect(await target.evaluate((element) => element === document.activeElement), "keyboard moved focus onto the control").toBe(true);
      expect(await computed(target), "keyboard focus equals forced :focus-visible").toEqual(await forced(page, target, "focus-visible"));
    });
  }
});

test.describe("form fields: forced states match real interaction", () => {
  test.use({ reducedMotion: "no-preference", viewport: { width: 1280, height: 900 } });

  const FIELDS: Array<{ name: string; locator: (page: Page) => Locator }> = [
    { name: "text field (time zone)", locator: (page) => page.locator(".display-settings-menu input").first() },
    { name: "select (language)", locator: (page) => page.locator(".display-settings-menu select").first() },
  ];

  for (const field of FIELDS) {
    test(`${field.name}: real hover and real click-focus equal the forced states`, async ({ page }) => {
      await page.goto("/");
      await page.waitForLoadState("networkidle");
      await page.getByRole("button", { name: /display settings/i }).click();
      const target = field.locator(page);
      await expect(target).toBeVisible();
      await page.mouse.move(2, 2);
      await settleTransitions(page);
      const resting = await computed(target);

      await target.hover();
      await settleTransitions(page);
      const realHover = await computed(target);
      await page.mouse.move(2, 2);
      await settleTransitions(page);
      expect(await computed(target), "leaving the field restores its resting style").toEqual(resting);
      expect(realHover, "real hover equals forced :hover").toEqual(await forced(page, target, "hover"));

      // A click focuses a field with :focus and :focus-visible (text input) and with :focus
      // (select, by browser heuristic); the forced focus state must equal whichever the
      // browser actually applied, so compare against the pseudo-classes that really matched.
      await target.click({ position: { x: 4, y: 4 } });
      await page.mouse.move(2, 2);
      await settleTransitions(page);
      const matches = await target.evaluate((element) => ({ focus: element.matches(":focus"), visible: element.matches(":focus-visible") }));
      expect(matches.focus, "the click focused the field").toBe(true);
      const realFocus = await computed(target);
      await target.evaluate((element) => (element as HTMLElement).blur());
      await settleTransitions(page);
      expect(realFocus, "real focus equals the forced focus state").toEqual(await forced(page, target, "focus-visible"));
    });
  }
});

test.describe("motion tokens drive real transitions", () => {
  test.use({ reducedMotion: "no-preference" });

  test("controls transition with the fast-effects spring and the shape uses the fast-spatial spring", async ({ page }) => {
    await page.goto("/");
    const tokens = await page.evaluate(() => {
      const root = getComputedStyle(document.documentElement);
      return {
        effects: root.getPropertyValue("--md-sys-motion-spring-fast-effects-duration").trim(),
        spatial: root.getPropertyValue("--md-sys-motion-spring-fast-spatial-duration").trim(),
        easing: root.getPropertyValue("--md-sys-motion-spring-fast-effects-easing").trim(),
      };
    });
    // Chrome serializes custom-property times canonically (".151s" for "151ms"), so compare in seconds.
    const toSeconds = (value: string): number => {
      const match = /^(\d*\.?\d+)(ms|s)$/.exec(value);
      if (!match) throw new Error(`not a time: ${value}`);
      return Number(match[1]) / (match[2] === "ms" ? 1000 : 1);
    };
    expect(tokens.easing).toMatch(/^linear\(/);
    const nav = page.getByRole("button", { name: "Tax simulator", exact: true });
    const timing = await nav.evaluate((element) => {
      const style = getComputedStyle(element);
      return { duration: style.transitionDuration, property: style.transitionProperty, easing: style.transitionTimingFunction };
    });
    const durations = timing.duration.split(",").map((part) => toSeconds(part.trim()));
    const close = (a: number, b: number) => Math.abs(a - b) < 0.0005;
    expect(durations.some((value) => close(value, toSeconds(tokens.effects))), "colour/background use the fast-effects spring").toBe(true);
    expect(durations.some((value) => close(value, toSeconds(tokens.spatial))), "corner shape uses the fast-spatial spring").toBe(true);
    expect(timing.property).toMatch(/background-color/);
    expect(timing.easing).toMatch(/linear\(/);
  });

  test("reduced motion removes every control transition", async ({ browser }) => {
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
