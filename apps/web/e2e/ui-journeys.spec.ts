import { expect, test, type Page } from "@playwright/test";
import { Buffer } from "node:buffer";
import axe from "axe-core";

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

test.describe("real-browser SPA journeys", () => {
  test("supports keyboard context and real lazy workspace loading", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText("Calculation service online")).toBeVisible();

    await page.keyboard.press("Tab");
    const skipLink = page.getByRole("link", { name: "Skip to workspace content" });
    await expect(skipLink).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.locator("#workspace-content")).toBeFocused();

    const workspaces = [
      ["Rule governance", "Review and promote a rule"],
      ["Replay operations", "Review replay gaps"],
      ["Vault releases", "Propose a vault release claim"],
      ["Vault reconciliation", "Vault reconciliation"],
      ["Settlement adjustments", "Propose an adjustment"],
      ["Refund claims", "Prepare a refund claim"],
      ["Refund executions", "Prepare execution"],
      ["Checkout authorization", "Review and authorize settlement"],
      ["Document quote", "Create persisted document evidence"],
      ["Document checkout", "Authorize a documented payment"],
    ] as const;

    for (const [buttonName, headingName] of workspaces) {
      await page.getByRole("button", { name: buttonName, exact: true }).click();
      await expect(page.getByRole("heading", { name: headingName, exact: true })).toBeVisible();
      await expect(page.locator("#workspace-content")).toBeFocused();
    }
  });

  test("has no rendered WCAG A/AA violations across the real SPA workspaces", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText("Calculation service online")).toBeVisible();
    await expectNoBrowserAxeViolations(page, "production quote");

    const workspaces = [
      ["Rule governance", "Review and promote a rule"],
      ["Replay operations", "Review replay gaps"],
      ["Vault releases", "Propose a vault release claim"],
      ["Vault reconciliation", "Vault reconciliation"],
      ["Settlement adjustments", "Propose an adjustment"],
      ["Refund claims", "Prepare a refund claim"],
      ["Refund executions", "Prepare execution"],
      ["Checkout authorization", "Review and authorize settlement"],
      ["Document quote", "Create persisted document evidence"],
      ["Document checkout", "Authorize a documented payment"],
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

    await expect(page.locator("html")).toHaveAttribute("dir", "rtl");
    await expect(page.locator(".app-shell")).toHaveAttribute("dir", "rtl");
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);

    const transitionDuration = await page.getByRole("button", { name: "Production quote" }).evaluate(
      (element) => getComputedStyle(element).transitionDuration,
    );
    expect(transitionDuration).toBe("0s");
  });

  test("contains a real dynamic-import failure without exposing exception data", async ({ page }) => {
    await page.route("**/assets/GovernancePanel-*.js", (route) => route.abort("failed"));
    await page.goto("/");
    await page.getByRole("button", { name: "Rule governance", exact: true }).click();

    await expect(page.getByRole("heading", { name: "This workspace could not be loaded" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Reload application" })).toBeVisible();
    await expect(page.getByText(/secret|exception|stack|token/i)).toHaveCount(0);
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
    await expect(page.getByText("No calculation yet", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Create quote", exact: true })).toBeDisabled();
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
    await expect(page.getByText("Calculation service online")).toBeVisible();
    await page.getByLabel("OIDC access token (memory-only)").fill("synthetic-browser-token");
    await page.getByLabel("Provider-signed jurisdiction evidence (JSON array)").fill("[{}]");
    await page.getByRole("button", { name: "Create quote", exact: true }).click();
    await expect(page.getByText("Calculation ID 01992f1b-4a70-7e30-8ea2-b6e37a9d9d21", { exact: true })).toBeVisible();

    await page.getByRole("button", { name: "Checkout authorization", exact: true }).click();
    await expect(page.getByRole("heading", { name: "Review and authorize settlement", exact: true })).toBeVisible();
    await expect(page.getByText(/expired; create a new quote/)).toBeVisible();
    await expect(page.getByRole("button", { name: "Compile for review", exact: true })).toBeDisabled();
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
    await expect(page.getByText("Calculation service online")).toBeVisible();
    await page.getByRole("button", { name: "Checkout authorization", exact: true }).click();
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
    await expect(page.getByText("Calculation service online")).toBeVisible();
    await page.getByRole("button", { name: "Checkout authorization", exact: true }).click();
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
    await expect(page.getByText("Calculation service online")).toBeVisible();
    await page.getByRole("button", { name: "Checkout authorization", exact: true }).click();
    await page.getByRole("button", { name: "Connect wallet", exact: true }).click();
    await expect(page.getByRole("combobox", { name: "Wallet account" })).toHaveValue("");
    await expectNoBrowserAxeViolations(page, "Wallet Standard account selection after reload");
  });

  test("fails closed when the authenticated document quote API is unavailable", async ({ page }) => {
    await page.route("**/api/v1/tax/document-quote", (route) => route.fulfill({
      status: 503,
      contentType: "application/json",
      headers: { "x-request-id": "01992f1b-4a70-7e30-8ea2-b6e37a9d9d31" },
      body: JSON.stringify({ error: { code: "SERVICE_UNAVAILABLE", message: "document quote service is unavailable" } }),
    }));

    await page.goto("/");
    await expect(page.getByText("Calculation service online")).toBeVisible();
    await page.getByLabel("OIDC access token (memory-only)").fill("synthetic-browser-token");
    await page.getByRole("button", { name: "Document quote", exact: true }).click();
    await page.getByLabel("Published rule set ID").fill("operator-document-flat-rate");
    await page.getByLabel("Provider-signed jurisdiction evidence (JSON array)").fill("[{}]");
    await page.getByRole("button", { name: "Create document quote", exact: true }).click();

    await expect(page.getByRole("alert")).toContainText("document quote service is unavailable");
    await expect(page.getByText("No persisted document quote yet", { exact: true })).toBeVisible();
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
    await expect(page.getByText("Calculation service online")).toBeVisible();
    await page.getByLabel("OIDC access token (memory-only)").fill("synthetic-browser-token");
    await page.getByRole("button", { name: "Document checkout", exact: true }).click();

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
    await expect(page.getByText("Calculation service online")).toBeVisible();
    await page.getByLabel("OIDC access token (memory-only)").fill("synthetic-browser-token");
    await page.getByRole("button", { name: "Document quote", exact: true }).click();
    await page.getByLabel("Published rule set ID").fill("operator-document-flat-rate");
    await page.getByLabel("Provider-signed jurisdiction evidence (JSON array)").fill("[{}]");
    await page.getByRole("button", { name: "Create document quote", exact: true }).click();
    await expect(page.getByText(`Calculation ID ${calculationId}`, { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Open document checkout", exact: true }).click();
    await expect(page.getByRole("heading", { name: "Authorize a documented payment", exact: true })).toBeVisible();

    await page.getByLabel("Issuer-signed document attestation (JSON)").fill('{"issuer":"synthetic-test"}');
    await page.getByRole("button", { name: "Verify document attestation", exact: true }).click();
    await expect(page.getByText("Document attested, not settled", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Compile document payment for review", exact: true })).toBeDisabled();

    await page.getByRole("button", { name: "Connect wallet", exact: true }).click();
    await page.getByLabel("Client nonce").fill("9");
    await page.getByLabel("Policy sequence").fill("3");
    await page.getByLabel("Policy digest (hex)").fill("00".repeat(32));
    await page.getByRole("button", { name: "Compile document payment for review", exact: true }).click();
    await expect(page.getByText("Server-derived payment authority", { exact: true })).toBeVisible();
    await expect(page.getByText("3000 atomic units", { exact: true })).toBeVisible();
    await expect(page.getByText("247 atomic units", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Sign and submit", exact: true })).toBeEnabled();
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
    await page.getByRole("button", { name: "Vault reconciliation", exact: true }).click();
    await page.getByLabel("OIDC access token (memory-only)").fill("synthetic-browser-token");
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
    await page.getByRole("button", { name: "Vault reconciliation", exact: true }).click();
    await page.getByLabel("OIDC access token (memory-only)").fill("synthetic-browser-token");
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
    await page.getByRole("button", { name: "Vault reconciliation", exact: true }).click();
    await page.getByLabel("OIDC access token (memory-only)").fill("synthetic-browser-token");
    await page.getByRole("button", { name: "Load reconciliation targets", exact: true }).click();

    await expect(page.getByRole("alert")).toContainText("vault reconciliation access is not authorized");
    await expect(page.getByText("Evidence not loaded", { exact: true })).toBeVisible();
    await expectNoBrowserAxeViolations(page, "vault reconciliation unauthorized state");
  });
});
