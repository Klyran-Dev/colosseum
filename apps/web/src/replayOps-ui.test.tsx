// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import ReplayOpsPanel from "./ReplayOpsPanel";

const run = {
  id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13",
  run_kind: "backfill",
  status: "completed",
  batch_size: 8,
  examined_count: 2,
  activated_count: 1,
  already_active_count: 1,
  failed_count: 0,
  failure_code: null,
  operator_approval_id: "01992f1b-4a70-7e31-8ea2-b6e37a9d9d13",
  started_at: "2026-10-05T10:00:00.000Z",
  completed_at: "2026-10-05T10:00:01.000Z",
};

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("route-verification operator evidence UI", () => {
  it("localizes recovery controls and preserves operator-safe language", () => {
    render(<ReplayOpsPanel apiBase="/api" serviceOnline accessToken="operator-token" locale="de-DE" />);

    expect(screen.getByRole("heading", { name: "Wiederherstellungslücken prüfen" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Wiederherstellungswarteschlange" })).toBeTruthy();
    expect(screen.getByText("Routenprüfungsnachweise", { exact: true })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Wiederherstellungselemente laden" })).toBeTruthy();
    expect(screen.queryByText(/Recovery never confirms that a payment moved/i)).toBeNull();
  });

  it("does not load route evidence until the operator explicitly asks", () => {
    const fetchMock = vi.fn();
    vi.stubGlobal("fetch", fetchMock);
    render(<ReplayOpsPanel apiBase="/api" serviceOnline accessToken="operator-token" />);

    expect(screen.getByRole("button", { name: "Load route-verification evidence" })).toBeTruthy();
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it("renders server-owned evidence as read-only operational status", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify([run]), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    render(<ReplayOpsPanel apiBase="/api" serviceOnline accessToken="operator-token" />);

    fireEvent.click(screen.getByRole("button", { name: "Load route-verification evidence" }));
    await waitFor(() => expect(screen.getByText("Verification completed")).toBeTruthy());
    expect(fetchMock.mock.calls[0][0]).toBe("/api/v1/indexer/route-verification-runs?limit=20");
    expect(screen.getByText(/does not activate a route or confirm finalized settlement/i)).toBeTruthy();
    expect(screen.getByText("01992f1b-4a70-7e31-8ea2-b6e37a9d9d13")).toBeTruthy();
  });

  it("fails closed when the evidence response is malformed", async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify([{ ...run, examined_count: 3 }]), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    render(<ReplayOpsPanel apiBase="/api" serviceOnline accessToken="operator-token" />);

    fireEvent.click(screen.getByRole("button", { name: "Load route-verification evidence" }));
    await waitFor(() => expect(screen.getByRole("alert").textContent).toMatch(/malformed/i));
    expect(screen.queryByText("Verification completed")).toBeNull();
  });

  it("redacts provider and transport exceptions from route-verification recovery copy", async () => {
    const providerDetail = "provider-internal route verification diagnostic";
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error(providerDetail)));
    render(<ReplayOpsPanel apiBase="/api" serviceOnline accessToken="operator-token" />);

    fireEvent.click(screen.getByRole("button", { name: "Load route-verification evidence" }));

    await waitFor(() => expect(screen.getByRole("alert").textContent).toContain("Route-verification evidence lookup failed"));
    expect(screen.getByRole("alert").textContent).not.toContain(providerDetail);
  });

  it("lists queued and dead-lettered gaps and offers recovery actions only for dead-lettered ones", async () => {
    const base = {
      stream_key: "mainnet/settlement",
      start_slot: "410",
      end_slot: "420",
      observed_finalized_slot: "430",
      reason: "subscription_gap",
      created_at: "2026-10-05T10:00:00.000Z",
      last_error_code: null,
      dead_lettered_at: null,
      operator_acknowledged_at: null,
      operator_acknowledged_by: null,
      operator_acknowledgement: null,
    };
    const gaps = [
      { ...base, id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d20", status: "queued", attempts: 1 },
      { ...base, id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d21", status: "dead_lettered", attempts: 8, last_error_code: "rpc_unavailable", dead_lettered_at: "2026-10-05T10:05:00.000Z" },
      {
        ...base,
        id: "01992f1b-4a70-7e30-8ea2-b6e37a9d9d22",
        status: "dead_lettered",
        attempts: 8,
        dead_lettered_at: "2026-10-05T10:06:00.000Z",
        operator_acknowledged_at: "2026-10-05T10:20:00.000Z",
        operator_acknowledged_by: "operator-1",
        operator_acknowledgement: "Reviewed against finalized RPC evidence; retry scheduled.",
      },
    ];
    const fetchMock = vi.fn().mockResolvedValue(new Response(JSON.stringify(gaps), { status: 200 }));
    vi.stubGlobal("fetch", fetchMock);
    const { container } = render(<ReplayOpsPanel apiBase="/api" serviceOnline accessToken="operator-token" />);

    fireEvent.change(screen.getByPlaceholderText("mainnet/settlement"), { target: { value: "mainnet/settlement" } });
    fireEvent.click(screen.getByRole("button", { name: "Load recovery items" }));
    await waitFor(() => expect(container.querySelectorAll(".replay-gap-card")).toHaveLength(3));

    expect(String(fetchMock.mock.calls[0][0])).toContain("/api/v1/indexer/replay-gaps");
    expect(container.querySelectorAll(".replay-action-box")).toHaveLength(2);
    expect(container.querySelectorAll(".replay-gap-card .evidence-note").length).toBeGreaterThanOrEqual(1);
  });
});
