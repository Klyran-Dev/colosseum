import { expect, test } from "@playwright/test";
import { axeViolationsInAllThemes } from "./axeAudit";

/**
 * Real touch input (ADR-0437). A touch device reports no hover capability, taps must
 * drive the same controls as clicks, a tap must not leave a hover style behind (mobile
 * browsers keep :hover on the tapped element), and the mobile layout gets the same
 * WCAG 2.2 AA audit as desktop in both themes.
 */
test.describe("touch devices", () => {
  test.use({ hasTouch: true, isMobile: true, viewport: { width: 375, height: 812 }, reducedMotion: "no-preference" });

  test("report no hover capability and a coarse pointer", async ({ page }) => {
    await page.goto("/");
    const capabilities = await page.evaluate(() => ({
      hover: window.matchMedia("(hover: hover)").matches,
      none: window.matchMedia("(hover: none)").matches,
      coarse: window.matchMedia("(pointer: coarse)").matches,
    }));
    expect(capabilities).toEqual({ hover: false, none: true, coarse: true });
  });

  test("taps open the mobile navigation and move between workspaces", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /open navigation/i }).tap();
    const simulator = page.getByRole("button", { name: "Tax simulator", exact: true });
    await expect(simulator).toBeVisible();
    await simulator.tap();
    await expect(page).toHaveURL(/\/operations\/tax-simulator$/);
    await expect(page.getByRole("heading", { level: 1, name: "Tax simulator" })).toBeVisible();
  });

  test("a tap does not leave hover styling behind", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");
    const targets = [page.locator(".system-status > summary"), page.locator(".account-menu-trigger"), page.locator(".display-settings > summary")];
    for (const target of targets) {
      const read = () => target.evaluate((element) => {
        const style = getComputedStyle(element);
        return { background: style.backgroundColor, color: style.color, border: style.borderTopColor };
      });
      const resting = await read();
      await target.tap();
      // Close any disclosure the tap opened so only hover could differ.
      await target.evaluate((element) => element.closest("details")?.removeAttribute("open"));
      await page.waitForFunction(() => document.getAnimations().length === 0);
      const afterTap = await read();
      expect(afterTap, "no hover or pressed style remains after the tap").toEqual(resting);
    }
  });

  test("the mobile layout meets WCAG 2.2 AA in both themes, with navigation open and popovers open", async ({ page }) => {
    await page.goto("/rules/access-approvals");
    await page.waitForLoadState("networkidle");
    await page.getByRole("button", { name: /open navigation/i }).tap();
    await page.evaluate(() => document.querySelectorAll(".product-shell .topbar details").forEach((element) => element.setAttribute("open", "")));
    const violations = await axeViolationsInAllThemes(page);
    expect(violations, "mobile WCAG 2.2 AA violations (dark and light)").toEqual([]);
  });
});
