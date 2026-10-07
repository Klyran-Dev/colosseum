import { expect, test } from "@playwright/test";

/**
 * WCAG 2.2 AA 1.4.10 Reflow: content must be usable at 320 CSS px wide (400% zoom
 * on a 1280px display) without two-dimensional scrolling. Every workspace is
 * checked in both themes; only regions that are intentionally scrollable (code and
 * evidence blocks) may exceed the viewport, and the page itself must not.
 */
const ROUTES = [
  "/operations/tax-quotes", "/operations/tax-simulator", "/operations/invoice-tax", "/operations/payment-review",
  "/operations/document-payment", "/rules/tax-rules", "/rules/access-approvals", "/money/treasury",
  "/money/balance-checks", "/money/partner-payouts", "/money/adjustments", "/money/refund-requests",
  "/money/refund-cancellations", "/money/refund-payments", "/records/exceptions-recovery", "/records/evidence-audit",
];

test.describe("reflow at 320 CSS px", () => {
  test.use({ viewport: { width: 320, height: 800 } });

  for (const route of ROUTES) {
    test(`${route} has no page-level horizontal scroll`, async ({ page }) => {
      await page.goto(route);
      await page.waitForLoadState("networkidle");
      // Menus and disclosures are part of the page: measure them open, not hidden.
      await page.evaluate(() => document.querySelectorAll(".product-shell details").forEach((element) => element.setAttribute("open", "")));
      for (const theme of ["dark", "light"]) {
        await page.evaluate((value: string) => {
          document.documentElement.setAttribute("data-theme", value);
          document.querySelector(".product-shell")?.setAttribute("data-theme", value);
        }, theme);
        const overflow = await page.evaluate(() => {
          const viewport = document.documentElement.clientWidth;
          const offenders = [...document.querySelectorAll<HTMLElement>(".product-shell *")]
            .filter((element) => {
              const box = element.getBoundingClientRect();
              if (box.width === 0 || box.height === 0) return false;
              // Intentionally scrollable regions contain their own overflow.
              if (element.closest("pre, code, [data-scrollable], .sr-only")) return false;
              for (let parent = element.parentElement; parent; parent = parent.parentElement) {
                const style = getComputedStyle(parent);
                if (/(auto|scroll|hidden|clip)/.test(style.overflowX)) return false;
              }
              return box.right > viewport + 1 || box.left < -1;
            })
            .slice(0, 8)
            .map((element) => `${element.tagName.toLowerCase()}.${String(element.className).split(" ").slice(0, 2).join(".")} ${Math.round(element.getBoundingClientRect().left)}..${Math.round(element.getBoundingClientRect().right)}`);
          const widest = offenders.length ? offenders : [...document.querySelectorAll<HTMLElement>(".product-shell *")]
            .filter((element) => element.getBoundingClientRect().right > viewport + 1 && !element.closest("pre, code, .sr-only"))
            .slice(0, 8)
            .map((element) => `${element.tagName.toLowerCase()}.${String(element.className).split(" ").slice(0, 2).join(".")} ${Math.round(element.getBoundingClientRect().left)}..${Math.round(element.getBoundingClientRect().right)}`);
          return { scrollWidth: document.documentElement.scrollWidth, viewport, offenders: document.documentElement.scrollWidth > viewport + 1 ? widest : offenders };
        });
        expect(overflow.offenders, `${route} (${theme}) elements outside the viewport`).toEqual([]);
        expect(overflow.scrollWidth, `${route} (${theme}) page scroll width vs ${overflow.viewport}px viewport`).toBeLessThanOrEqual(overflow.viewport + 1);
      }
    });
  }
});
