import { expect, test } from "@playwright/test";
import { interactiveCandidates, verifyInteractiveKind, type SweepResult } from "./interactionSupport";

/**
 * Replaces sampling with a sweep (ADR-0438). Across every workspace, in the closed
 * state and with popovers open, take each distinct kind of enabled interactive
 * element (tag, role, and class set) once, and require real mouse hover and a real
 * press to equal the forced :hover and :active states used by the style snapshots.
 * A kind whose real interaction cannot reach it (covered by another element) is
 * counted, not hidden, and the skip rate is bounded.
 */
const ROUTES = [
  "/operations/tax-quotes", "/operations/tax-simulator", "/operations/invoice-tax", "/operations/payment-review",
  "/operations/document-payment", "/rules/tax-rules", "/rules/access-approvals", "/money/treasury",
  "/money/balance-checks", "/money/partner-payouts", "/money/adjustments", "/money/refund-requests",
  "/money/refund-cancellations", "/money/refund-payments", "/records/exceptions-recovery", "/records/evidence-audit",
];
const MAX_KINDS = 160;

test.describe("interaction sweep", () => {
  test.use({ reducedMotion: "no-preference", viewport: { width: 1280, height: 900 } });

  test("real hover and press equal the forced states for every distinct interactive kind", async ({ page }) => {
    test.setTimeout(300_000);
    const seen = new Set<string>();
    const result: SweepResult = { verified: 0, skipped: [], mismatches: [] };

    for (const route of ROUTES) {
      await page.goto(route);
      await page.waitForLoadState("networkidle");
      for (const mode of ["closed", "popovers open"] as const) {
        if (mode === "popovers open") {
          await page.evaluate(() => document.querySelectorAll(".product-shell details").forEach((element) => element.setAttribute("open", "")));
        }
        for (const candidate of await interactiveCandidates(page)) {
          if (seen.has(candidate.signature) || seen.size >= MAX_KINDS) continue;
          const outcome = await verifyInteractiveKind(page, candidate, `${route} [${mode}] ${candidate.signature}`, result);
          if (outcome !== "hidden") seen.add(candidate.signature);
        }
      }
    }

    expect(result.mismatches, "forced states must equal real interaction").toEqual([]);
    // Signed-out workspaces expose few enabled controls; the account menu and
    // workspace-gate sign-in action are intentionally absent until a session
    // exists (ADR-0450, ADR-0459). The fixture sweep (style-fixtures.spec.ts)
    // covers the enabled primary buttons, fields, and links of every captured
    // state.
    expect(result.verified, "number of distinct interactive kinds verified").toBeGreaterThanOrEqual(8);
    expect(result.skipped.length / Math.max(1, result.verified + result.skipped.length), `skipped (unreachable) kinds: ${result.skipped.join("; ")}`).toBeLessThanOrEqual(0.25);
    console.log(`INTERACTION_SWEEP verified=${result.verified} skipped=${result.skipped.length} kinds=${seen.size}\n${result.skipped.join("\n")}`);
  });
});
