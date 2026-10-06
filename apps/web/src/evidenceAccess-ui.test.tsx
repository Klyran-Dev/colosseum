// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import EvidenceAccessPanel from "./EvidenceAccessPanel";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const responseEvent = {
  id: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9012",
  calculation_id: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9013",
  actor_subject: "operator@example.test",
  request_id: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9014",
  purpose: "tax_quote_replay",
  outcome: "allowed",
  facts_digest: Array.from({ length: 32 }, (_, index) => index),
  occurred_at: "2026-10-05T10:00:00.000Z",
};

describe("evidence access explorer workspace", () => {
  it("starts empty and does not enable protected reads without a session", () => {
    render(<EvidenceAccessPanel apiBase="/api" serviceOnline accessToken="" />);
    expect(screen.getByRole("heading", { name: "Review evidence access" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Load access history" }).hasAttribute("disabled")).toBe(true);
    expect(screen.getByText("No access history loaded")).toBeTruthy();
  });

  it("loads a server-shaped record and keeps technical evidence behind disclosure", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => new Response(JSON.stringify([responseEvent]), { status: 200 })));
    render(<EvidenceAccessPanel apiBase="/api" serviceOnline accessToken="opaque-token" />);
    fireEvent.click(screen.getByRole("button", { name: "Load access history" }));
    expect(await screen.findByText("Access allowed")).toBeTruthy();
    expect(screen.getByText("operator@example.test")).toBeTruthy();
    expect(screen.getByText("Technical record")).toBeTruthy();
    const technicalDetails = screen.getByText("Technical record").closest("details");
    expect(technicalDetails?.hasAttribute("open")).toBe(false);
    fireEvent.click(screen.getByText("Technical record"));
    await waitFor(() => expect(screen.getByText(responseEvent.id)).toBeTruthy());
  });
});
