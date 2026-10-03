// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import SettlementAdjustmentPanel from "./SettlementAdjustmentPanel";
import adjustmentVector from "../../../test-vectors/settlement-adjustment-evidence-v1.json";

const record = {
  id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
  merchant_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d11",
  idempotency_key: "refund-01",
  request_digest: Array(32).fill(1),
  adjustment_digest: Array(32).fill(2),
  original_settlement_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d14",
  asset_id: Array(32).fill(4),
  kind: "Refund",
  delta: { gross_atomic: "-250", tax_atomic: "-25", net_atomic: "-225" },
  reason_code: "customer_refund",
  evidence_digest: Array(32).fill(5),
  source_calculation_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d15",
  replacement_calculation_id: null,
  status: "proposed",
  proposer_subject: "proposer",
  approver_subject: null,
};

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("settlement adjustment operator workspace", () => {
  it("submits exact signed strings and renders the server-owned proposal", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify(record), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    render(<SettlementAdjustmentPanel apiBase="/api" serviceOnline accessToken="proposer-token" />);

    fireEvent.click(screen.getByRole("button", { name: "Create adjustment request" }));
    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    const body = JSON.parse(fetchMock.mock.calls[0][1].body as string) as Record<string, unknown>;
    expect(fetchMock.mock.calls[0][0]).toBe("/api/v1/settlement/adjustments");
    expect(body.delta).toEqual({ gross_atomic: "-250", tax_atomic: "-25", net_atomic: "-225" });
    expect(body.asset_id).toEqual(Array(32).fill(4));
    expect(fetchMock.mock.calls[0][1].headers).toMatchObject({ authorization: "Bearer proposer-token" });
    expect(screen.getByText(/is proposed/i)).toBeTruthy();
    expect(screen.queryByRole("option", { name: "posted" })).toBeNull();
  });

  it("loads, approves, and refreshes through the governed API without claiming payment", async () => {
    const approved = { ...record, status: "approved", approver_subject: "approver" };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(record), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ status: "transitioned", adjustment_id: record.id, from_status: "proposed", to_status: "approved", approver_subject: "approver" }), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(approved), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    render(<SettlementAdjustmentPanel apiBase="/api" serviceOnline accessToken="approver-token" />);

    fireEvent.change(screen.getByLabelText("Adjustment reference"), { target: { value: record.id } });
    fireEvent.click(screen.getByRole("button", { name: "Load adjustment" }));
    await waitFor(() => expect(screen.getByText("proposed")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "Record approved decision" }));

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(3));
    expect(fetchMock.mock.calls[1][0]).toBe(`/api/v1/settlement/adjustments/${record.id}/transition`);
    expect(JSON.parse(fetchMock.mock.calls[1][1].body as string)).toEqual({ to_status: "approved", evidence: { review_ticket: "" } });
    expect(screen.getByText(/No token movement occurred/i)).toBeTruthy();
  });

  it("shows bounded API failures and preserves the no-client-fallback boundary", async () => {
    const requestId = "01992f1b-4a70-7e30-8ea2-b6e37a9d9d16";
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify({ error: { message: "adjustment rejected" } }), { status: 409, headers: { "x-request-id": requestId } }));
    vi.stubGlobal("fetch", fetchMock);
    render(<SettlementAdjustmentPanel apiBase="/api" serviceOnline accessToken="proposer-token" />);

    fireEvent.click(screen.getByRole("button", { name: "Create adjustment request" }));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toMatch(/adjustment rejected/));
    expect(screen.getByRole("alert").textContent).toContain(requestId);
    expect(screen.queryByText(/payment succeeded/i)).toBeNull();
  });

  it("prepares the server-owned wallet record only after approval", async () => {
    const approved = { ...record, status: "approved", approver_subject: "approver" };
    const compileResponse = {
      status: "prepared_unsubmitted",
      intent_id: record.id,
      chain_intent_id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d16",
      request_digest: Array(32).fill(1),
      instruction_digest: Array(32).fill(2),
      registry: adjustmentVector.expected.adjustment_registry,
      registry_slot: "12",
      registry_commitment: "finalized",
      registry_account_digest: Array(32).fill(3),
      instruction: adjustmentVector.expected,
    };
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(approved), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(compileResponse), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    render(<SettlementAdjustmentPanel apiBase="/api" serviceOnline accessToken="approver-token" />);

    fireEvent.change(screen.getByLabelText("Adjustment reference"), { target: { value: record.id } });
    fireEvent.click(screen.getByRole("button", { name: "Load adjustment" }));
    await waitFor(() => expect(screen.getByText("approved")).toBeTruthy());
    fireEvent.click(screen.getByRole("button", { name: "Prepare ledger record" }));

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(2));
    expect(fetchMock.mock.calls[1][0]).toBe(`/api/v1/settlement/adjustments/${record.id}/compile`);
    expect(JSON.parse(fetchMock.mock.calls[1][1].body as string)).toEqual({ confirm: true });
    expect(screen.getByText(/ready for wallet signing/i)).toBeTruthy();
    expect(screen.queryByText(/payment succeeded/i)).toBeNull();
  });
});
