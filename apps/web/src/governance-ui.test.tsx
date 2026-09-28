// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import GovernancePanel from "./GovernancePanel";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("governance workbench submission boundary", () => {
  it("submits the structured canonical rule document through the authenticated API", async () => {
    const draftResponse = {
      rule_set_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d15",
      external_id: "new-sales-tax",
      version: 1,
      jurisdiction: "US-CA",
      regime: "sales-tax",
      status: "draft",
      rule_digest: Array.from({ length: 32 }, (_, index) => index),
      effective_from: "2026-09-27T00:00:00.000Z",
      effective_until: null,
      governance_revision: 0,
    };
    const governanceResponse = {
      rule_set_id: draftResponse.rule_set_id,
      external_id: draftResponse.external_id,
      version: 1,
      jurisdiction: "US-CA",
      regime: "sales-tax",
      status: "draft",
      rule_digest_hex: "ab".repeat(32),
      author_subject: "author",
      reviewer_subject: null,
      approver_subject: null,
      governance_revision: 0,
      effective_from: draftResponse.effective_from,
      effective_until: null,
      events: [],
    };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(draftResponse), { status: 201 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(governanceResponse), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);

    render(<GovernancePanel apiBase="/api" serviceOnline />);
    fireEvent.change(screen.getAllByLabelText("OIDC access token (memory-only)")[0], { target: { value: "author-token" } });
    fireEvent.click(screen.getByRole("button", { name: "Create governed draft" }));

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
    const requestBody = JSON.parse(fetchMock.mock.calls[0][1].body as string) as Record<string, unknown>;
    const ruleSet = requestBody.rule_set as Record<string, unknown>;
    const component = (ruleSet.components as Array<Record<string, unknown>>)[0];

    expect(fetchMock.mock.calls[0][0]).toBe("/api/v1/tax/rules");
    expect(requestBody.authorization).toBeUndefined();
    expect(ruleSet.id).toBe("new-sales-tax");
    expect(ruleSet.version).toBe(1);
    expect(component.rate).toEqual({ Flat: { numerator: "10", denominator: "100" } });
    expect(component.base).toEqual({ sources: ["Net"] });
    expect(component.credit_atomic).toBe("0");
    expect(fetchMock.mock.calls[0][1].headers).toMatchObject({ authorization: "Bearer author-token" });
    expect(screen.getByText(/Draft new-sales-tax v1 was created/)).toBeTruthy();
  });

  it("simulates the server-persisted rule after the governance snapshot is loaded", async () => {
    const draftResponse = {
      rule_set_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d15",
      external_id: "new-sales-tax",
      version: 1,
      jurisdiction: "US-CA",
      regime: "sales-tax",
      status: "draft",
      rule_digest: Array.from({ length: 32 }, (_, index) => index),
      effective_from: "2026-09-27T00:00:00.000Z",
      effective_until: null,
      governance_revision: 0,
    };
    const governanceResponse = {
      rule_set_id: draftResponse.rule_set_id,
      external_id: draftResponse.external_id,
      version: 1,
      jurisdiction: "US-CA",
      regime: "sales-tax",
      status: "draft",
      rule_digest_hex: "ab".repeat(32),
      author_subject: "author",
      reviewer_subject: null,
      approver_subject: null,
      governance_revision: 0,
      effective_from: draftResponse.effective_from,
      effective_until: null,
      events: [],
    };
    const simulationResponse = {
      calculation_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d16",
      status: "simulated_unattested",
      result: {
        rule_set_id: "new-sales-tax",
        rule_version: 1,
        engine_version: "0.1.0",
        currency: "USD",
        jurisdiction: "US-CA",
        facts_digest: Array.from({ length: 32 }, (_, index) => index),
        rule_digest: Array.from({ length: 32 }, (_, index) => 255 - index),
        gross_atomic: "1100",
        total_tax_atomic: "100",
        total_net_atomic: "1000",
        components: [{ id: "sales-tax", label: "Sales tax", base_atomic: "1000", tax_atomic: "100", explanation: "10% of net" }],
      },
    };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(draftResponse), { status: 201 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(governanceResponse), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(simulationResponse), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);

    render(<GovernancePanel apiBase="/api" serviceOnline />);
    fireEvent.change(screen.getAllByLabelText("OIDC access token (memory-only)")[0], { target: { value: "author-token" } });
    fireEvent.click(screen.getByRole("button", { name: "Create governed draft" }));
    await waitFor(() => expect((screen.getByRole("button", { name: "Simulate persisted rule" }) as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByRole("button", { name: "Simulate persisted rule" }));

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(3));
    expect(fetchMock.mock.calls[2][0]).toBe(`/api/v1/tax/rules/${draftResponse.rule_set_id}/simulate`);
    expect(JSON.parse(fetchMock.mock.calls[2][1].body as string)).toEqual({
      facts: { gross_atomic: "1100", net_atomic: "1000", currency: "USD", jurisdiction: "US-CA" },
    });
    expect(screen.getByText(/unattested server result/i)).toBeTruthy();
  });
});
