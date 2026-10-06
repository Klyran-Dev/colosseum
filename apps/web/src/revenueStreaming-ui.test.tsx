// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import RevenueStreamingPanel from "./RevenueStreamingPanel";
import { AuthorizationSnapshot } from "./authorization";

const destinationId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d21";
const splitPlanId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d22";
const policyId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d23";
const planId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d24";
const publicKey = "11111111111111111111111111111111";

const snapshot: AuthorizationSnapshot = {
  state: "server-allowed",
  subject: "payout-proposer",
  tenantId: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d25",
  roles: ["revenue_stream_proposer"],
  scopes: ["tax:stream:read", "tax:stream:propose"],
  attributes: {},
  capabilities: {
    "revenue_stream.read": { kind: "allow", reasonCode: "scope_and_role_satisfied", reason: "The service confirmed this action for the current account." },
    "revenue_stream.propose": { kind: "allow", reasonCode: "scope_and_role_satisfied", reason: "The service confirmed this action for the current account." },
    "revenue_stream.govern": { kind: "deny", reasonCode: "role_missing", reason: "Approval responsibility is not assigned." },
    "revenue_stream.claim.prepare": { kind: "deny", reasonCode: "role_missing", reason: "Payout preparation responsibility is not assigned." },
  },
};

const catalog = {
  destinations: [{
    id: destinationId,
    route_key: "partner-primary",
    version: 1,
    recipient_owner: publicKey,
    mint: publicKey,
    token_program: publicKey,
    verification_slot: "42",
  }],
  split_plans: [{
    id: splitPlanId,
    plan_key: "partner-share",
    version: 2,
    mint: publicKey,
  }],
  policies: [{
    id: policyId,
    rule_set_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d26",
    rule_external_id: "india-goods",
    rule_version: 4,
    jurisdiction: "IN",
    regime: "GST",
    effective_from: "2026-01-01T00:00:00.000Z",
    effective_until: null,
  }],
};

const plan = {
  id: planId,
  status: "proposed",
  proposer_subject: "payout-proposer",
  approver_subject: null,
  accrued_total_atomic: "0",
  released_total_atomic: "0",
  next_nonce: "1",
  destination_route_id: destinationId,
  spec: {
    operator_subject: "payout-operator",
    start_slot: "100",
    end_slot: null,
    max_total_atomic: "1000",
    max_claim_atomic: "100",
    settlement_route_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d27",
    asset: Array(32).fill(3),
    recipient: Array(32).fill(4),
    plan_digest: Array(32).fill(1),
    policy_digest: Array(32).fill(2),
  },
};

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("partner payout production selection flow", () => {
  it("uses server-owned references instead of asking operators to paste technical digests", async () => {
    const fetchMock = vi.fn().mockImplementation((input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url.endsWith("/v1/revenue-streaming/plans") && init?.method !== "POST") return Promise.resolve(new Response("[]", { status: 200 }));
      if (url.endsWith("/v1/revenue-streaming/references")) return Promise.resolve(new Response(JSON.stringify(catalog), { status: 200 }));
      if (url.endsWith("/v1/revenue-streaming/plans") && init?.method === "POST") return Promise.resolve(new Response(JSON.stringify(plan), { status: 200 }));
      throw new Error(`Unexpected request: ${url}`);
    });
    vi.stubGlobal("fetch", fetchMock);

    render(<RevenueStreamingPanel accessToken="proposer-token" authorizationSnapshot={snapshot} authorizationError={null} />);

    await waitFor(() => expect(screen.getByRole("heading", { name: "Create a payout plan" })).toBeTruthy());
    expect(screen.getByRole("combobox", { name: /Verified payout destination/ })).toBeTruthy();
    expect(screen.getByRole("combobox", { name: /Settlement split/ })).toBeTruthy();
    expect(screen.getByRole("combobox", { name: /Published tax policy/ })).toBeTruthy();
    expect(screen.queryByLabelText(/fingerprint/i)).toBeNull();
    expect(screen.queryByText(/Advanced rule references/i)).toBeNull();

    fireEvent.change(screen.getByLabelText(/Who prepares payouts\?/), { target: { value: "payout-operator" } });
    fireEvent.change(screen.getByLabelText("Start after settlement"), { target: { value: "100" } });
    fireEvent.change(screen.getByLabelText(/Maximum lifetime payout/), { target: { value: "1000" } });
    fireEvent.change(screen.getByLabelText("Maximum payout at one time"), { target: { value: "100" } });
    fireEvent.click(screen.getByRole("button", { name: "Create plan for review" }));

    await waitFor(() => expect(fetchMock.mock.calls.some((call) => String(call[0]).endsWith("/v1/revenue-streaming/plans") && call[1]?.method === "POST")).toBe(true));
    const post = fetchMock.mock.calls.find((call) => String(call[0]).endsWith("/v1/revenue-streaming/plans") && call[1]?.method === "POST");
    expect(JSON.parse(String(post?.[1]?.body))).toMatchObject({
      destination_route_id: destinationId,
      split_plan_id: splitPlanId,
      policy_id: policyId,
    });
    expect(JSON.parse(String(post?.[1]?.body))).not.toHaveProperty("plan_digest");
    expect(JSON.parse(String(post?.[1]?.body))).not.toHaveProperty("policy_digest");
  });

  it("localizes the payout operator surface without translating authority values", async () => {
    vi.stubGlobal("fetch", vi.fn().mockImplementation((input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/v1/revenue-streaming/plans")) return Promise.resolve(new Response("[]", { status: 200 }));
      if (url.endsWith("/v1/revenue-streaming/references")) return Promise.resolve(new Response(JSON.stringify(catalog), { status: 200 }));
      throw new Error(`Unexpected request: ${url}`);
    }));

    render(<RevenueStreamingPanel accessToken="payout-token" authorizationSnapshot={snapshot} authorizationError={null} locale="de-DE" />);

    await waitFor(() => expect(screen.getByRole("heading", { name: "Auszahlungsplan erstellen" })).toBeTruthy());
    expect(screen.getByRole("heading", { name: "Partnerauszahlungspläne" })).toBeTruthy();
    const destination = screen.getByRole("combobox", { name: /Verifiziertes Auszahlungsziel/ }) as HTMLSelectElement;
    expect(destination.value).toBe(destinationId);
    expect(screen.getByText("Prüfung erforderlich")).toBeTruthy();
  });

  it("redacts provider and transport exceptions from plan loading", async () => {
    const providerDetail = "provider-internal payout plan diagnostic";
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error(providerDetail)));

    render(<RevenueStreamingPanel accessToken="payout-token" authorizationSnapshot={snapshot} authorizationError={null} />);

    await waitFor(() => expect(screen.getAllByRole("alert").some((alert) => alert.textContent?.includes("Payout plans could not be loaded.") ?? false)).toBe(true));
    expect(screen.getAllByRole("alert").map((alert) => alert.textContent).join(" ")).not.toContain(providerDetail);
  });
});
