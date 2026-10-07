// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { PublicKey } from "@solana/web3.js";
import { Buffer } from "buffer";
import { afterEach, describe, expect, it, vi } from "vitest";
import settlementVector from "../../../test-vectors/settlement-instruction-v1.json";
import CheckoutPanel from "./CheckoutPanel";

// The wallet boundary is exercised by the wallet tests and the real-browser
// journeys; this file only needs a connected payer to reach the compiled review.
vi.mock("./WalletPicker", () => ({
  default: ({ onConnected }: { onConnected?: (publicKey: PublicKey) => void }) => (
    <button type="button" onClick={() => onConnected?.(new PublicKey("11111111111111111111111111111111"))}>Connect test wallet</button>
  ),
}));

const routeId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13";
const route = {
  id: routeId,
  route_key: "local-usdc",
  version: 1,
  merchant_account: "8qbHbw2BbbTHBW1sbeqakYXVKRQM8Ne7pLK7m6CVfeR",
  mint: "CktRuQ2mttgRGkXJtyksdKHjUdc2C4TgDzyB98oEzy8",
  mint_decimals: 6,
  tax_vault: "8nGRRJuGoubpUKHVa5paPCvGLQU39nnu3zFWYREbSeNN",
  net_destination: "QWmroo4YnnMqYW3cnxWkFdaTxGD3P7vMSzwMHGbUzwF",
  token_program: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
};
const plan = {
  id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d42",
  plan_key: "matching-plan",
  version: 1,
  merchant_account: route.merchant_account,
  mint: route.mint,
  plan_digest: Array.from({ length: 32 }, (_, index) => index),
  recipients: [
    { token_account: route.net_destination, basis_points: 7000 },
    { token_account: route.tax_vault, basis_points: 3000 },
  ],
};

const vectorInstruction = {
  ...settlementVector.expected,
  instruction_data_base64: settlementVector.expected.instruction_data_base64_chunks.join(""),
  compute_budget_policy: { policy_version: 1, compute_unit_limit: 200_000, micro_lamports_per_compute_unit: 1_000 },
};
const splitArtifact = {
  ...vectorInstruction,
  instruction_data_base64: Buffer.concat([Buffer.from("A6qY2V4JCdU=", "base64"), Buffer.alloc(80)]).toString("base64"),
  accounts: [
    ...vectorInstruction.accounts.slice(0, 8),
    { ...vectorInstruction.accounts[7], is_writable: false },
    vectorInstruction.accounts[9],
    vectorInstruction.accounts[10],
    vectorInstruction.accounts[11],
    vectorInstruction.accounts[12],
    { ...vectorInstruction.accounts[8], is_writable: true },
    { ...vectorInstruction.accounts[2], is_writable: true },
  ],
  split_plan: vectorInstruction.accounts[7].pubkey,
  split_plan_digest: [25, ...new Array(31).fill(0)],
  intent_id: routeId,
  request_digest: new Array(32).fill(1),
  instruction_digest: new Array(32).fill(2),
  route_id: routeId,
  split_plan_id: plan.id,
};

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

describe("split-plan payment review", () => {
  it("shows the bound recipient distribution after the payment is prepared", async () => {
    vi.stubGlobal("fetch", vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/v1/settlement/routes")) return new Response(JSON.stringify([route]), { status: 200 });
      if (url.endsWith("/v1/settlement/split-plans")) return new Response(JSON.stringify([plan]), { status: 200 });
      if (url.endsWith("/v1/settlement/split/compile-from-quote")) return new Response(JSON.stringify(splitArtifact), { status: 200 });
      return new Response(null, { status: 404 });
    }));
    const calculation = {
      calculation_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d21",
      status: "calculated_unattested",
      expires_at_unix: Math.floor(Date.now() / 1000) + 3600,
      result: {
        rule_set_id: "published-sales-tax", rule_version: 3, engine_version: "tax-engine/v1", atomic_decimals: 2, currency: "USD", jurisdiction: "US-CA",
        facts_digest: Array(32).fill(1), rule_digest: Array(32).fill(2), gross_atomic: "1000", total_tax_atomic: "100", total_net_atomic: "900",
        components: [],
      },
    };
    const { container } = render(
      <CheckoutPanel apiBase="/api" serviceOnline accessToken="token" calculation={calculation as never} formatAtomic={(value, currency) => `${currency} ${value}`} />,
    );

    const mode = await waitFor(() => {
      const select = container.querySelector('select option[value="split"]')?.parentElement as HTMLSelectElement | null;
      if (!select) throw new Error("mode select not ready");
      return select;
    });
    fireEvent.change(mode, { target: { value: "split" } });
    await waitFor(() => expect(container.querySelector(".route-evidence.full-field")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "Connect test wallet" }));
    const prepare = screen.getByRole("button", { name: "Prepare payment for review" }) as HTMLButtonElement;
    await waitFor(() => expect(prepare.disabled).toBe(false));
    fireEvent.click(prepare);

    await waitFor(() => expect(container.querySelector(".split-review")).toBeTruthy());
    expect(container.querySelector(".split-review")?.textContent).toMatch(/2/);
  });
});
