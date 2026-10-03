import { expect, test, type Page } from "@playwright/test";
import { Buffer } from "node:buffer";
import axe from "axe-core";
import { PublicKey } from "@solana/web3.js";

async function expectNoBrowserAxeViolations(
  page: Page,
  surface: string,
): Promise<void> {
  const violations = await page.evaluate(async (axeSource) => {
    if (!("axe" in window)) {
      const script = document.createElement("script");
      script.textContent = axeSource;
      document.head.appendChild(script);
    }
    const browserAxe = (window as unknown as {
      axe?: {
        run: (context: Document, options: { runOnly: { type: "tag"; values: string[] } }) => Promise<{
          violations: Array<{
            id: string;
            impact: string | null;
            help: string;
            nodes: Array<{ target: string[]; html: string }>;
          }>;
        }>;
      };
    }).axe;
    if (!browserAxe) throw new Error("axe-core did not initialize in the real browser");
    const result = await browserAxe.run(document, {
      runOnly: { type: "tag", values: ["wcag2a", "wcag2aa"] },
    });
    return result.violations.map(({ id, impact, help, nodes }) => ({
      id,
      impact,
      help,
      nodes: nodes.map(({ target, html }) => ({ target, html })),
    }));
  }, axe.source);

  expect(violations, `${surface} has real-browser axe violations`).toEqual([]);
}

const CAPABILITY_IDS = [
  "access.control",
  "evidence.replay",
  "refund.claim.cancel",
  "refund.claim.create",
  "refund.payment.release",
  "settlement.adjustment",
  "settlement.document.wallet",
  "settlement.wallet",
  "tax.document.quote.create",
  "tax.preview.run",
  "tax.quote.create",
  "tax.rule.govern",
  "vault.reconciliation",
  "vault.release",
] as const;

function completeCapabilityCatalog(overrides: Record<string, { decision: "allow" | "deny" | "indeterminate"; reason_code: string }>) {
  return Object.fromEntries(CAPABILITY_IDS.map((id) => [
    id,
    overrides[id] ?? { decision: "deny", reason_code: "scope_missing" },
  ]));
}

test.describe("real-browser SPA journeys", () => {
  test("supports direct workspace links and browser history", async ({ page }) => {
    await page.goto("/money/refund-payments");
    await expect(page).toHaveURL(/\/money\/refund-payments$/);
    await expect(page.getByRole("heading", { name: "Prepare refund release", exact: true })).toBeVisible();

    await page.getByRole("button", { name: "Balance checks", exact: true }).click();
    await expect(page).toHaveURL(/\/money\/balance-checks$/);
    await expect(page.getByRole("heading", { name: "Check tax balances", exact: true })).toBeVisible();

    await page.goBack();
    await expect(page).toHaveURL(/\/money\/refund-payments$/);
    await expect(page.getByRole("heading", { name: "Prepare refund release", exact: true })).toBeVisible();
    await page.goForward();
    await expect(page).toHaveURL(/\/money\/balance-checks$/);
    await expectNoBrowserAxeViolations(page, "addressable workspace navigation");
  });

  test("supports keyboard context and real lazy workspace loading", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText("All systems operational")).toBeVisible();
    await expect(page.getByText("Local development", { exact: true })).toBeVisible();
    await expect(page.getByText("Local Solana ledger", { exact: true })).toBeVisible();
    await expect(page.getByText("Acme Treasury", { exact: true })).toHaveCount(0);
    await expect(page.getByText("Test network", { exact: true })).toHaveCount(0);

    await page.keyboard.press("Tab");
    const skipLink = page.getByRole("link", { name: "Skip to workspace content" });
    await expect(skipLink).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.locator("#workspace-content")).toBeFocused();

    const workspaces = [
      ["Tax rules", "Review and publish a rule"],
      ["Exceptions & recovery", "Recovery queue"],
      ["Treasury", "Create a release request"],
      ["Balance checks", "Check tax balances"],
      ["Adjustments", "Create an adjustment request"],
      ["Refund requests", "Prepare a refund request"],
      ["Refund payments", "Prepare refund release"],
      ["Payment review", "Review payment before signing"],
      ["Invoice tax", "Calculate tax for an invoice"],
      ["Document payment", "Authorize a documented payment"],
    ] as const;

    for (const [buttonName, headingName] of workspaces) {
      await page.getByRole("button", { name: buttonName, exact: true }).click();
      await expect(page.getByRole("heading", { name: headingName, exact: true })).toBeVisible();
      await expect(page.locator("#workspace-content")).toBeFocused();
    }
  });

  test("replaces the verified tenant context when the opaque session changes", async ({ page }) => {
    const tenantA = {
      token: "tenant-a-session",
      subject: "operator-a",
      tenantId: "01900000-0000-4000-8000-000000000001",
      snapshotId: "01900000-0000-7000-8000-000000000001",
      capabilities: completeCapabilityCatalog({
        "access.control": { decision: "allow", reason_code: "scope_and_role_satisfied" },
        "tax.preview.run": { decision: "allow", reason_code: "scope_and_role_satisfied" },
      }),
    };
    const tenantB = {
      token: "tenant-b-session",
      subject: "operator-b",
      tenantId: "02900000-0000-4000-8000-000000000002",
      snapshotId: "02900000-0000-7000-8000-000000000002",
      capabilities: completeCapabilityCatalog({
        "access.control": { decision: "allow", reason_code: "scope_and_role_satisfied" },
        "tax.preview.run": { decision: "deny", reason_code: "scope_missing" },
      }),
    };
    await page.route("**/api/v1/authorization/capabilities", async (route) => {
      const authorization = route.request().headers().authorization ?? "";
      const selected = authorization.includes(tenantB.token) ? tenantB : tenantA;
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          schema: "authorization-capabilities/v1",
          snapshot_id: selected.snapshotId,
          subject: selected.subject,
          tenant_id: selected.tenantId,
          expires_at_unix: Math.floor(Date.now() / 1000) + 30,
          capabilities: selected.capabilities,
        }),
      });
    });

    await page.goto("/");
    await expect(page.getByText("All systems operational")).toBeVisible();
    const credential = page.getByLabel("Sign-in credential (held in memory)");
    await credential.fill(tenantA.token);
    await expect(page.getByText("Account 01900000", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Access & approvals", exact: true }).click();
    await expect(page.getByRole("heading", { name: "Who can do what", exact: true })).toBeVisible();
    await expect(page.getByText("operator-a", { exact: true })).toBeVisible();
    await expect(page.getByText("Available", { exact: true }).first()).toBeVisible();

    await credential.fill(tenantB.token);
    await expect(page.getByText("Account 02900000", { exact: true })).toBeVisible();
    await expect(page.getByText("Account 01900000", { exact: true })).toHaveCount(0);
    await expect(page.getByText("operator-b", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Tax simulator", exact: true })).toHaveAttribute("title", /Not available/);
    await expectNoBrowserAxeViolations(page, "multi-tenant capability session reset");
  });

  test("has no rendered WCAG A/AA violations across the real SPA workspaces", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText("All systems operational")).toBeVisible();
    await expectNoBrowserAxeViolations(page, "production quote");

    const workspaces = [
      ["Tax rules", "Review and publish a rule"],
      ["Exceptions & recovery", "Recovery queue"],
      ["Treasury", "Create a release request"],
      ["Balance checks", "Check tax balances"],
      ["Adjustments", "Create an adjustment request"],
      ["Refund requests", "Prepare a refund request"],
      ["Refund payments", "Prepare refund release"],
      ["Payment review", "Review payment before signing"],
      ["Invoice tax", "Calculate tax for an invoice"],
      ["Document payment", "Authorize a documented payment"],
    ] as const;

    for (const [buttonName, headingName] of workspaces) {
      await page.getByRole("button", { name: buttonName, exact: true }).click();
      await expect(page.getByRole("heading", { name: headingName, exact: true })).toBeVisible();
      await expectNoBrowserAxeViolations(page, buttonName);
    }
  });

  test("keeps narrow RTL and reduced-motion display behavior bounded", async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 800 });
    await page.goto("/");

    await page.getByText("Display settings", { exact: true }).click();
    await page.getByRole("combobox", { name: "Language" }).selectOption("ar-EG");
    await page.getByRole("button", { name: "Open navigation", exact: true }).click();
    await expect(page.getByRole("button", { name: "Tax rules", exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Balance checks", exact: true }).click();
    await expect(page.getByRole("button", { name: "Open navigation", exact: true })).toHaveAttribute("aria-expanded", "false");
    await page.getByRole("button", { name: "Open navigation", exact: true }).click();

    await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
    await expect(page.locator(".app-shell")).toHaveAttribute("dir", "rtl");
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);

    const transitionDuration = await page.getByRole("button", { name: "Tax quotes" }).evaluate(
      (element) => getComputedStyle(element).transitionDuration,
    );
    expect(transitionDuration).toBe("0s");
  });

  test("contains a real dynamic-import failure without exposing exception data", async ({ page }) => {
    await page.route("**/assets/GovernancePanel-*.js", (route) => route.abort("failed"));
    await page.goto("/");
    await page.getByRole("button", { name: "Tax rules", exact: true }).click();

    await expect(page.getByRole("heading", { name: "This workspace could not be loaded" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Reload application" })).toBeVisible();
    await expect(page.locator(".workspace-error").getByText(/secret|exception|stack|token/i)).toHaveCount(0);
  });

  test("fails closed when the API health boundary is unavailable", async ({ page }) => {
    await page.route("**/*healthz*", (route) => route.fulfill({
      status: 503,
      contentType: "application/json",
      body: JSON.stringify({ error: { code: "SERVICE_UNAVAILABLE", message: "temporarily unavailable" } }),
    }));
    await page.goto("/");

    const outageAlert = page.getByRole("alert");
    await expect(outageAlert).toHaveText("The service is unavailable. No client-side fallback is used.");
    await expect(page.getByText("Your result will appear here", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Prepare quote", exact: true })).toBeDisabled();
    await expectNoBrowserAxeViolations(page, "API unavailable state");
  });

  test("shows an expired quote and blocks wallet compilation before signing", async ({ page }) => {
    const now = Math.floor(Date.now() / 1000);
    await page.route("**/api/v1/tax/quote", (route) => route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        calculation_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d21",
        status: "calculated_unattested",
        expires_at_unix: now - 1,
        result: {
          rule_set_id: "published-sales-tax",
          rule_version: 3,
          engine_version: "tax-engine/v1",
          currency: "USD",
          jurisdiction: "US-CA",
          facts_digest: Array(32).fill(1),
          rule_digest: Array(32).fill(2),
          gross_atomic: "1000",
          total_tax_atomic: "100",
          total_net_atomic: "900",
          components: [{
            id: "sales-tax",
            label: "Sales tax",
            base_atomic: "1000",
            effective_base_atomic: "1000",
            threshold_atomic: null,
            exemption_applied: false,
            raw_tax: { numerator: "100", denominator: "1" },
            credit_atomic: "0",
            clamped_tax_atomic: "100",
            tax_atomic: "100",
            explanation: "Published rule evidence",
          }],
        },
      }),
    }));
    await page.route("**/api/v1/settlement/routes", (route) => route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify([{
        id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d22",
        route_key: "local-usdc",
        version: 1,
        merchant_account: "8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR",
        mint: "CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8",
        mint_decimals: 6,
        tax_vault: "8nGRRJuGoubpUKHVa5paPCvGLQU39nnu3zFWYREbSeNN",
        net_destination: "QWmroo4YnnMqYW3cnxWkFdaTxGD3P7vMSzwMHGbUzwF",
        token_program: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
      }]),
    }));
    await page.route("**/api/v1/settlement/split-plans", (route) => route.fulfill({
      status: 200,
      contentType: "application/json",
      body: "[]",
    }));

    await page.goto("/");
    await expect(page.getByText("All systems operational")).toBeVisible();
    await page.getByLabel("Sign-in credential (held in memory)").fill("synthetic-browser-token");
    await page.getByLabel("Location evidence").fill("[{}]");
    await page.getByLabel("Total before tax").fill("1100");
    await page.getByLabel("Amount subject to tax").fill("1000");
    await page.getByLabel("Currency").fill("USD");
    await page.getByLabel("Approved tax rule").fill("operator-simulator-flat-rate");
    await page.getByLabel("Rule version").fill("1");
    await page.getByLabel("Jurisdiction").fill("US-CA");
    await page.getByLabel("Tax category").fill("sales-tax");
    await page.getByRole("button", { name: "Prepare quote", exact: true }).click();
    await expect(page.getByText("Reference 01992f1b-4a70-7e30-8ea2-b6e37a9d9d21", { exact: true })).toBeVisible();

    await page.getByRole("button", { name: "Payment review", exact: true }).click();
    await expect(page.getByRole("heading", { name: "Review payment before signing", exact: true })).toBeVisible();
    await expect(page.getByText(/expired; create a new quote/)).toBeVisible();
    await expect(page.getByRole("button", { name: "Prepare payment for review", exact: true })).toBeDisabled();
    await expectNoBrowserAxeViolations(page, "expired quote checkout state");
  });

  test("renders an accessible wallet-rejection state without claiming authorization", async ({ page }) => {
    await page.addInitScript(() => {
      Object.defineProperty(window, "solana", {
        configurable: true,
        value: {
          connect: async () => { throw new Error("User rejected wallet connection"); },
        },
      });
    });

    await page.goto("/");
    await expect(page.getByText("All systems operational")).toBeVisible();
    await page.getByRole("button", { name: "Payment review", exact: true }).click();
    await page.getByRole("button", { name: "Connect wallet", exact: true }).click();
    await expect(page.getByRole("alert")).toContainText("User rejected wallet connection");
    await expect(page.getByText("No wallet connected", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "wallet rejection state");
  });

  test("requires explicit Wallet Standard account selection across a browser reload", async ({ page }) => {
    await page.addInitScript(() => {
      const alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
      const decodeBase58 = (value: string): Uint8Array => {
        const bytes = [0];
        for (const character of value) {
          const index = alphabet.indexOf(character);
          if (index < 0) throw new Error("invalid test public key");
          let carry = index;
          for (let offset = 0; offset < bytes.length; offset += 1) {
            const next = bytes[offset] * 58 + carry;
            bytes[offset] = next & 0xff;
            carry = next >> 8;
          }
          while (carry > 0) {
            bytes.push(carry & 0xff);
            carry >>= 8;
          }
        }
        for (let index = 0; value[index] === "1"; index += 1) bytes.push(0);
        return Uint8Array.from(bytes.reverse());
      };

      const accounts = [
        "11111111111111111111111111111111",
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
      ].map((address) => ({
        address,
        publicKey: decodeBase58(address),
        chains: ["solana:localnet"],
        features: ["solana:signTransaction"],
      }));
      const wallet = {
        version: "1.0.0",
        name: "Browser Standard Test Wallet",
        icon: "data:image/svg+xml;base64,PHN2Zy8+",
        chains: ["solana:localnet"],
        accounts,
        features: {
          "standard:connect": {
            version: "1.0.0",
            connect: async () => ({ accounts }),
          },
          "solana:signTransaction": {
            version: "1.0.0",
            signTransaction: async () => [],
          },
        },
      };
      window.addEventListener("wallet-standard:app-ready", (event) => {
        const detail = (event as CustomEvent<{ register: (candidate: typeof wallet) => void }>).detail;
        detail.register(wallet);
      });
    });

    await page.goto("/");
    await expect(page.getByText("All systems operational")).toBeVisible();
    await page.getByRole("button", { name: "Payment review", exact: true }).click();
    await page.getByRole("button", { name: "Connect wallet", exact: true }).click();

    const accountPicker = page.getByRole("combobox", { name: "Wallet account" });
    await expect(accountPicker).toBeVisible();
    await expect(page.getByRole("alert")).toContainText("multiple Solana accounts");
    await expect(accountPicker).toHaveValue("");
    await accountPicker.selectOption("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
    await page.getByRole("button", { name: "Connect selected account", exact: true }).click();
    await expect(page.getByText("Selected account: TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "Wallet Standard account selection");

    await page.reload();
    await expect(page.getByText("All systems operational")).toBeVisible();
    await page.getByRole("button", { name: "Payment review", exact: true }).click();
    await page.getByRole("button", { name: "Connect wallet", exact: true }).click();
    await expect(page.getByRole("combobox", { name: "Wallet account" })).toHaveValue("");
    await expectNoBrowserAxeViolations(page, "Wallet Standard account selection after reload");
  });

  test("renders and reloads server-bound finalized refund evidence", async ({ page }) => {
    const response = finalizedRefundClaimResponse();
    await page.route("**/api/v1/settlement/refund-claims**", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(response),
      });
    });

    await page.goto("/");
    await expect(page.getByText("All systems operational")).toBeVisible();
    await page.getByLabel("Sign-in credential (held in memory)").fill("synthetic-browser-token");
    await page.getByRole("button", { name: "Refund requests", exact: true }).click();
    await page.getByLabel("Approved correction reference").fill("01992f1b-4a70-7e30-8ea2-b6e37a9d9d12");
    await page.getByRole("button", { name: "Prepare refund request", exact: true }).click();

    await expect(page.getByText("Finalized claim event evidence matched the durable artifact.", { exact: true })).toBeVisible();
    await page.getByText("Finalized event proof", { exact: true }).click();
    await expect(page.getByText("AdjustmentRefundClaimCreated · v1", { exact: true })).toBeVisible();
    await expect(page.getByText("Gross / tax / refund", { exact: true }).locator("..").getByText("100 / 25 / 75", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "finalized refund proof");

    await page.reload();
    await expect(page.getByText("All systems operational")).toBeVisible();
    await page.getByLabel("Sign-in credential (held in memory)").fill("synthetic-browser-token");
    await page.getByRole("button", { name: "Refund requests", exact: true }).click();
    await page.getByLabel("Refund request reference").fill("01992f1b-4a70-7e30-8ea2-b6e37a9d9d14");
    await page.getByRole("button", { name: "Load saved request", exact: true }).click();
    await expect(page.getByText("Finalized claim event evidence matched the durable artifact.", { exact: true })).toBeVisible();
    await page.getByText("Finalized event proof", { exact: true }).click();
    await expect(page.getByText("77", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "finalized refund proof after reload");
  });

  test("fails closed when the authenticated document quote API is unavailable", async ({ page }) => {
    await page.route("**/api/v1/tax/document-quote", (route) => route.fulfill({
      status: 503,
      contentType: "application/json",
      headers: { "x-request-id": "01992f1b-4a70-7e30-8ea2-b6e37a9d9d31" },
      body: JSON.stringify({ error: { code: "SERVICE_UNAVAILABLE", message: "document quote service is unavailable" } }),
    }));

    await page.goto("/");
    await expect(page.getByText("All systems operational")).toBeVisible();
    await page.getByLabel("Sign-in credential (held in memory)").fill("synthetic-browser-token");
    await page.getByRole("button", { name: "Invoice tax", exact: true }).click();
    await page.getByLabel("Document ID").fill("invoice-1");
    await page.getByLabel("Currency").fill("USD");
    await page.getByLabel("Jurisdiction").fill("US-CA");
    await page.getByLabel("Tax regime").fill("sales-tax");
    await page.getByLabel("Published rule set ID").fill("operator-document-flat-rate");
    await page.getByLabel("Rule version").fill("1");
    await page.getByLabel("Line ID").fill("line-a");
    await page.getByLabel("Amount before tax").fill("3000");
    await page.getByLabel("Amount after tax").fill("3000");
    await page.getByLabel("Location evidence").fill("[{}]");
    await page.getByRole("button", { name: "Calculate invoice tax", exact: true }).click();

    await expect(page.getByRole("alert")).toContainText("document quote service is unavailable");
    await expect(page.getByText("No invoice calculation yet", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Open document checkout", exact: true })).toHaveCount(0);
    await expectNoBrowserAxeViolations(page, "document quote API unavailable state");
  });

  test("renders document checkout route failure without enabling signing", async ({ page }) => {
    await page.route("**/api/v1/settlement/routes*", (route) => route.fulfill({
      status: 503,
      contentType: "application/json",
      headers: { "x-request-id": "01992f1b-4a70-7e30-8ea2-b6e37a9d9d32" },
      body: JSON.stringify({ error: { code: "SERVICE_UNAVAILABLE", message: "active settlement routes are unavailable" } }),
    }));

    await page.goto("/");
    await expect(page.getByText("All systems operational")).toBeVisible();
    await page.getByLabel("Sign-in credential (held in memory)").fill("synthetic-browser-token");
    await page.getByRole("button", { name: "Document payment", exact: true }).click();

    await expect(page.getByRole("alert")).toContainText("active settlement routes are unavailable");
    await expect(page.getByRole("combobox", { name: "Active settlement route" })).toHaveValue("");
    await expect(page.getByRole("button", { name: "Compile document payment for review", exact: true })).toBeDisabled();
    await expect(page.getByRole("button", { name: "Sign and submit", exact: true })).toHaveCount(0);
    await expectNoBrowserAxeViolations(page, "document checkout route failure state");
  });

  test("proves document evidence handoff and keeps signing behind a reviewed artifact", async ({ page }) => {
    const calculationId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d41";
    const routeId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d42";
    const intentId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d43";
    const payer = "cGfHiC6Kgg3FpFZvgwGcswsCRtp4aBP2fzuXRQPizuN";
    const programId = "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi";
    const tokenProgram = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
    const route = {
      id: routeId,
      route_key: "local-usdc",
      version: 1,
      merchant_account: "8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR",
      mint: "GgBaCs3NCBuZN12kCJgAW63ydqohFkHEdfdEXBPzLHq",
      mint_decimals: 6,
      tax_vault: "LbUiWL3xVV8hTFYBVdbTNrpDo41NKS6o3LHHuDzjfcY",
      net_destination: "QWmroo4YnnMqYW3cnxWkFdaTxGD3P7vMSzwMHGbUzwF",
      token_program: tokenProgram,
    };
    const expiresAt = Math.floor(Date.now() / 1000) + 600;
    const instructionData = Buffer.concat([
      Buffer.from("XAF7GP/zWDU=", "base64"),
      Buffer.alloc(40 + 32),
      Buffer.alloc(32, 8),
      Buffer.alloc(32, 9),
    ]).toString("base64");
    const accounts = [
      { pubkey: payer, is_signer: true, is_writable: true },
      { pubkey: "qcfm6jd37nhqmbY6JomQWtdD9S4HieqnmRm49Hum6ir", is_signer: false, is_writable: false },
      { pubkey: route.merchant_account, is_signer: false, is_writable: false },
      { pubkey: "CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8", is_signer: false, is_writable: true },
      { pubkey: route.mint, is_signer: false, is_writable: false },
      { pubkey: route.tax_vault, is_signer: false, is_writable: true },
      { pubkey: "CsNjEE9EUZUJeAVqSWmbZxT9WB9h4xYvuLNRRPymrGcs", is_signer: false, is_writable: false },
      { pubkey: "3MUvGdb23Wh2N42NKSiQE1wcRbR3RXSXZde2MPiihHAq", is_signer: false, is_writable: false },
      { pubkey: route.net_destination, is_signer: false, is_writable: true },
      { pubkey: "8nGRRJuGoubpUKHVa5paPCvGLQU39nnu3zFWYREbSeNN", is_signer: false, is_writable: true },
      { pubkey: tokenProgram, is_signer: false, is_writable: false },
      { pubkey: "11111111111111111111111111111111", is_signer: false, is_writable: false },
    ];

    await page.addInitScript(({ address }) => {
      const alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
      const decodeBase58 = (value: string): Uint8Array => {
        const bytes = [0];
        for (const character of value) {
          const index = alphabet.indexOf(character);
          if (index < 0) throw new Error("invalid test public key");
          let carry = index;
          for (let offset = 0; offset < bytes.length; offset += 1) {
            const next = bytes[offset] * 58 + carry;
            bytes[offset] = next & 0xff;
            carry = next >> 8;
          }
          while (carry > 0) {
            bytes.push(carry & 0xff);
            carry >>= 8;
          }
        }
        for (let index = 0; value[index] === "1"; index += 1) bytes.push(0);
        return Uint8Array.from(bytes.reverse());
      };
      const account = {
        address,
        publicKey: decodeBase58(address),
        chains: ["solana:localnet"],
        features: ["solana:signTransaction"],
      };
      const wallet = {
        version: "1.0.0",
        name: "Browser Document Test Wallet",
        icon: "data:image/svg+xml;base64,PHN2Zy8+",
        chains: ["solana:localnet"],
        accounts: [account],
        features: {
          "standard:connect": { version: "1.0.0", connect: async () => ({ accounts: [account] }) },
          "solana:signTransaction": { version: "1.0.0", signTransaction: async () => [] },
        },
      };
      window.addEventListener("wallet-standard:app-ready", (event) => {
        const detail = (event as CustomEvent<{ register: (candidate: typeof wallet) => void }>).detail;
        detail.register(wallet);
      });
    }, { address: payer });

    await page.route("**/api/v1/tax/document-quote", (routeRequest) => routeRequest.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        calculation_id: calculationId,
        status: "attested",
        expires_at_unix: expiresAt,
        schema: "solana-tax-protocol/tax-document/v1",
        result: {
          document_id: "invoice-1",
          rule_set_id: "operator-document-flat-rate",
          rule_version: 1,
          engine_version: "tax-engine/v1",
          currency: "USD",
          jurisdiction: "US-CA",
          facts_digest: new Array(32).fill(1),
          rule_digest: new Array(32).fill(2),
          gross_atomic: "3000",
          total_tax_atomic: "247",
          total_net_atomic: "2753",
          lines: [{
            id: "line-a",
            gross_atomic: "3000",
            total_tax_atomic: "247",
            total_net_atomic: "2753",
            components: [{ id: "sales-tax", label: "Sales tax", tax_atomic: "247", explanation: "Server-owned document trace" }],
          }],
        },
      }),
    }));
    await page.route("**/api/v1/settlement/routes", (routeRequest) => routeRequest.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify([route]),
    }));
    await page.route(`**/api/v1/tax/document-quotes/${calculationId}/attestation/verify`, (routeRequest) => routeRequest.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        status: "document_attested_unsettled",
        calculation_id: calculationId,
        payload_digest: new Array(32).fill(8),
        issuer_public_key: new Array(32).fill(9),
        expires_at_unix: expiresAt,
      }),
    }));
    await page.route("**/api/v1/settlement/document/compile-from-quote", (routeRequest) => routeRequest.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        status: "prepared_unsubmitted",
        program_id: programId,
        instruction_data_base64: instructionData,
        accounts,
        protocol_config: accounts[1].pubkey,
        vault_authority: accounts[6].pubkey,
        policy_attestation: accounts[7].pubkey,
        receipt: accounts[9].pubkey,
        compute_budget_policy: { policy_version: 1, compute_unit_limit: 200000, micro_lamports_per_compute_unit: 1000 },
        document_attestation_digest: new Array(32).fill(8),
        payment_scope_digest: new Array(32).fill(9),
        intent_id: intentId,
        request_digest: new Array(32).fill(1),
        instruction_digest: new Array(32).fill(2),
        route_id: routeId,
        authorization: {
          calculation_id: calculationId,
          route_id: routeId,
          source_attestation_digest: new Array(32).fill(8),
          payment_scope_digest: new Array(32).fill(9),
          scope: { kind: "full_document" },
          gross_atomic: "3000",
          tax_atomic: "247",
          net_atomic: "2753",
        },
      }),
    }));

    await page.goto("/");
    await expect(page.getByText("All systems operational")).toBeVisible();
    await page.getByLabel("Sign-in credential (held in memory)").fill("synthetic-browser-token");
    await page.getByRole("button", { name: "Invoice tax", exact: true }).click();
    await page.getByLabel("Document ID").fill("invoice-1");
    await page.getByLabel("Currency").fill("USD");
    await page.getByLabel("Jurisdiction").fill("US-CA");
    await page.getByLabel("Tax regime").fill("sales-tax");
    await page.getByLabel("Published rule set ID").fill("operator-document-flat-rate");
    await page.getByLabel("Rule version").fill("1");
    await page.getByLabel("Line ID").fill("line-a");
    await page.getByLabel("Amount before tax").fill("3000");
    await page.getByLabel("Amount after tax").fill("3000");
    await page.getByLabel("Location evidence").fill("[{}]");
    await page.getByRole("button", { name: "Calculate invoice tax", exact: true }).click();
    await expect(page.getByText(`Calculation ID ${calculationId}`, { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Open document checkout", exact: true }).click();
    await expect(page.getByRole("heading", { name: "Authorize a documented payment", exact: true })).toBeVisible();

    await page.getByLabel("Issuer-signed document attestation (JSON)").fill('{"issuer":"synthetic-test"}');
    await page.getByRole("button", { name: "Verify document attestation", exact: true }).click();
    await expect(page.getByText("Document attested, not settled", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Compile document payment for review", exact: true })).toBeDisabled();

    await page.getByRole("button", { name: "Connect wallet", exact: true }).click();
    await page.getByRole("button", { name: "Compile document payment for review", exact: true }).click();
    await expect(page.getByText("Payment ready for review", { exact: true })).toBeVisible();
    await expect(page.getByText("3000", { exact: true })).toBeVisible();
    await expect(page.getByText("247", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Sign and send payment", exact: true })).toBeEnabled();
    await expectNoBrowserAxeViolations(page, "document evidence handoff and reviewed artifact");
  });

  test("renders server-owned reconciliation evidence and stops on mismatch", async ({ page }) => {
    const target = {
      id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d12",
      merchant_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d11",
      route_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
      stream_key: "mainnet/settlement/v1",
      mint: "mint-account",
      token_program: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
      tax_vault: "vault-account",
      opening_balance_atomic: "1000",
      opening_balance_slot: "20",
      opening_reference: "opening ceremony INC-42",
      receipt_tax_atomic: "250",
      release_atomic: "100",
    };
    const latestRun = {
      id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d14",
      status: "mismatch",
      opening_balance_atomic: "1000",
      receipt_tax_atomic: "250",
      release_atomic: "100",
      expected_balance_atomic: "1150",
      observed_balance_atomic: "1149",
      observed_slot: "99",
      error_code: null,
      created_at: "2026-09-27T00:00:00.000Z",
      stale: false,
    };
    const historyRun = { ...latestRun, id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d15", status: "matched", observed_balance_atomic: "1150" };

    await page.route("**/v1/vault/reconciliation/targets/*/runs*", async (route) => {
      await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify([historyRun, latestRun]) });
    });
    await page.route("**/v1/vault/reconciliation/targets?*", async (route) => {
      await route.fulfill({ status: 200, contentType: "application/json", body: JSON.stringify([{ target, latest_run: latestRun }]) });
    });

    await page.goto("/");
    await page.getByRole("button", { name: "Balance checks", exact: true }).click();
    await page.getByLabel("Sign-in credential (held in memory)").fill("synthetic-browser-token");
    await page.getByRole("button", { name: "Load reconciliation targets", exact: true }).click();

    await expect(page.getByText("Mismatch — stop and investigate", { exact: true }).first()).toBeVisible();
    await expect(page.getByText("The latest finalized observation does not match the evidence-derived expectation.", { exact: false })).toBeVisible();
    await expect(page.getByText("opening ceremony INC-42", { exact: true })).toBeVisible();

    await page.getByRole("button", { name: /vault-account.*Mismatch/, exact: false }).click();
    await expect(page.getByRole("heading", { name: "Recent runs", exact: true })).toBeVisible();
    await expect(page.getByText("matched", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "vault reconciliation mismatch state");
  });

  test("does not render an old matched observation as current evidence", async ({ page }) => {
    const staleRun = {
      id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d14",
      status: "matched",
      opening_balance_atomic: "1000",
      receipt_tax_atomic: "250",
      release_atomic: "100",
      expected_balance_atomic: "1150",
      observed_balance_atomic: "1150",
      observed_slot: "99",
      error_code: null,
      created_at: "2026-09-27T00:00:00.000Z",
      stale: true,
    };
    const target = {
      id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d12",
      merchant_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d11",
      route_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
      stream_key: "mainnet/settlement/v1",
      mint: "mint-account",
      token_program: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
      tax_vault: "vault-account",
      opening_balance_atomic: "1000",
      opening_balance_slot: "20",
      opening_reference: "opening ceremony INC-42",
      receipt_tax_atomic: "250",
      release_atomic: "100",
    };
    await page.route("**/v1/vault/reconciliation/targets*", (route) => route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify([{ target, latest_run: staleRun }]),
    }));

    await page.goto("/");
    await page.getByRole("button", { name: "Balance checks", exact: true }).click();
    await page.getByLabel("Sign-in credential (held in memory)").fill("synthetic-browser-token");
    await page.getByRole("button", { name: "Load reconciliation targets", exact: true }).click();

    await expect(page.getByText("Stale — stop and investigate", { exact: true }).first()).toBeVisible();
    await expect(page.getByText("The latest finalized observation is stale and cannot prove the current vault balance.", { exact: false })).toBeVisible();
    await expect(page.getByText("The latest finalized observation does not match the evidence-derived expectation.", { exact: false })).toHaveCount(0);
    await expectNoBrowserAxeViolations(page, "vault reconciliation stale state");
  });

  test("fails closed on unauthorized reconciliation reads", async ({ page }) => {
    await page.route("**/v1/vault/reconciliation/targets*", (route) => route.fulfill({
      status: 403,
      contentType: "application/json",
      headers: { "x-request-id": "01992f1b-4a70-7e30-8ea2-b6e37a9d9d16" },
      body: JSON.stringify({ error: { code: "FORBIDDEN", message: "vault reconciliation access is not authorized" } }),
    }));

    await page.goto("/");
    await page.getByRole("button", { name: "Balance checks", exact: true }).click();
    await page.getByLabel("Sign-in credential (held in memory)").fill("synthetic-browser-token");
    await page.getByRole("button", { name: "Load reconciliation targets", exact: true }).click();

    await expect(page.getByRole("alert")).toContainText("vault reconciliation access is not authorized");
    await expect(page.getByText("Balance evidence not loaded", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "vault reconciliation unauthorized state");
  });
});

function finalizedRefundClaimResponse(): Record<string, unknown> {
  const instruction = {
    status: "prepared_unsubmitted",
    program_id: "4vJ9JU1bJJE96FWSJKvHsmmFADCg4gpZQff4P3bkLKi",
    instruction_data_base64: "eYjG9eQLZXMDAAAAAAAAAAkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJANJJawAAAABY1ElrAAAAAA==",
    accounts: [
      { pubkey: "11157t3sqMV725NVRLrVQbAu98Jjfk1uCKehJnXXQs", is_signer: true, is_writable: true },
      { pubkey: "1117mWrzzrZr312ebPDHu8tbfMwFNvCvMbr6WepCNG", is_signer: true, is_writable: false },
      { pubkey: "1119DWteoLSdjvrT6g6L8C2PfDD2faiTQUpsjY2RiF", is_signer: false, is_writable: false },
      { pubkey: "111BuZ6b86gm7XhxjvTakhRvxSMjXp2GqgifkNUmDK", is_signer: false, is_writable: false },
      { pubkey: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", is_signer: false, is_writable: false },
      { pubkey: "qcfm6jd37nhqmbY6JomQWtdD9S4HieqnmRm49Hum6ir", is_signer: false, is_writable: false },
      { pubkey: "4fuGSLS5aka44BPDqoH7RSg1HLvy4bMU4zY551u2Dnts", is_signer: false, is_writable: false },
      { pubkey: "Fk3Z5sqgXWRYu3dFuQqshGmgGJoSxQSnxTHSFt9CKM5T", is_signer: false, is_writable: false },
      { pubkey: "111FJo4zLAGU9nzTWa6EnbV4VAmtG4FR8kcokrtZYr", is_signer: false, is_writable: false },
      { pubkey: "111A4R8Vf7xP4Rz7ZkYd8H7f4Q5J4h4Y4V4w4U4T4S", is_signer: false, is_writable: false },
      { pubkey: "HL3Hhyp81PHGgfQQXmAbJy7ZmEi9MJX3ex4UFAunH2g5", is_signer: false, is_writable: false },
      { pubkey: "111B4R8Vf7xP4Rz7ZkYd8H7f4Q5J4h4Y4V4w4U4T4S", is_signer: false, is_writable: false },
      { pubkey: "111C4R8Vf7xP4Rz7ZkYd8H7f4Q5J4h4Y4V4w4U4T4S", is_signer: false, is_writable: false },
      { pubkey: "4DqbTd6EtCebWaXDpCRmoSRVjeHB1k6B3CFYQ89dunAK", is_signer: false, is_writable: true },
      { pubkey: "11111111111111111111111111111111", is_signer: false, is_writable: false },
    ],
    registry: "4fuGSLS5aka44BPDqoH7RSg1HLvy4bMU4zY551u2Dnts",
    adjustment: "Fk3Z5sqgXWRYu3dFuQqshGmgGJoSxQSnxTHSFt9CKM5T",
    original_receipt: "111FJo4zLAGU9nzTWa6EnbV4VAmtG4FR8kcokrtZYr",
    claim: "4DqbTd6EtCebWaXDpCRmoSRVjeHB1k6B3CFYQ89dunAK",
    claim_nonce: "3",
  };
  return {
    status: "finalized",
    intent_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d14",
    request_digest: Array(32).fill(3),
    instruction_digest: Array(32).fill(1),
    artifact_digest: Array(32).fill(2),
    claim: instruction.claim,
    artifact: {
      schema: "refund-movement-artifact/v1",
      instruction,
      instruction_digest: Array(32).fill(1),
      artifact_digest: Array(32).fill(2),
      refund_destination: instruction.accounts[12].pubkey,
      merchant_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d11",
      adjustment_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d12",
      funding_configuration_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
      receipt_signature: "5VfY7WmYcR7h9o9E9nP7pQ8rS",
      receipt_slot: "42",
      proposer_subject: "proposer",
      refund_funder_subject: "refund-funder",
      proposer: instruction.accounts[0].pubkey,
      refund_funder: instruction.accounts[1].pubkey,
      merchant: instruction.accounts[2].pubkey,
      mint: instruction.accounts[3].pubkey,
      token_program: instruction.accounts[4].pubkey,
      tax_vault: instruction.accounts[9].pubkey,
      net_source: instruction.accounts[11].pubkey,
      gross_atomic: "100",
      tax_atomic: "25",
      net_atomic: "75",
      execute_after_unix: 1800000000,
      expires_at_unix: 1800000600,
    },
    signed_transaction_digest: null,
    signature: "5VfY7WmYcR7h9o9E9nP7pQ8rS",
    submitted_slot: "77",
    commitment: "finalized",
    last_error_code: null,
    finalized_evidence: {
      event_type: "AdjustmentRefundClaimCreated",
      event_version: "v1",
      event_discriminator: Array(8).fill(4),
      commitment: "finalized",
      signature: "5VfY7WmYcR7h9o9E9nP7pQ8rS",
      slot: "77",
      event_index: 2,
      claim: Array.from(new PublicKey(instruction.claim).toBytes()),
      adjustment: Array(32).fill(1),
      original_receipt: Array(32).fill(2),
      merchant: Array(32).fill(3),
      mint: Array(32).fill(4),
      nonce: "3",
      gross_magnitude: "100",
      tax_magnitude: "25",
      net_magnitude: "75",
      refund_funder: Array(32).fill(5),
      refund_destination: Array(32).fill(6),
      proposer: Array(32).fill(7),
      reference_digest: Array(32).fill(8),
      execute_after_unix: 1800000000,
      expires_at_unix: 1800000600,
      evidence_digest: Array(32).fill(1),
    },
  };
}
