// @vitest-environment happy-dom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import { WORKSPACE_POLICIES } from "./authorization";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("server-owned authorization shell boundary", () => {
  it("loads the capability snapshot with the bearer token and reflects the verified result", async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      if (url.endsWith("/healthz")) return new Response(null, { status: 204 });
      if (url.endsWith("/v1/authorization/capabilities")) {
        return new Response(JSON.stringify({
          schema: "authorization-capabilities/v1",
          snapshot_id: "01900000-0000-7000-8000-000000000001",
          subject: "operator-1",
          tenant_id: "01900000-0000-4000-8000-000000000001",
          expires_at_unix: Math.floor(Date.now() / 1000) + 30,
          capabilities: Object.fromEntries(WORKSPACE_POLICIES.map((policy) => [policy.id, {
            decision: policy.id === "tax.quote.create" ? "indeterminate" : "allow",
            reason_code: policy.id === "tax.quote.create" ? "resource_context_required" : "scope_and_role_satisfied",
          }])),
        }), { status: 200, headers: { "content-type": "application/json" } });
      }
      return new Response(null, { status: 404 });
    });
    vi.stubGlobal("fetch", fetchMock);

    render(<App />);
    fireEvent.change(screen.getByRole("textbox", { name: /Sign-in credential/ }), { target: { value: "signed-session" } });

    await waitFor(() => expect(screen.getAllByText("Access checked").length).toBeGreaterThan(0));
    const capabilityCall = fetchMock.mock.calls.find(([input]) => String(input).endsWith("/v1/authorization/capabilities"));
    expect(capabilityCall?.[1]?.headers).toMatchObject({ authorization: "Bearer signed-session" });

    fireEvent.click(screen.getByRole("button", { name: /^Tax rules$/ }));
    await waitFor(() => expect(screen.getByRole("heading", { name: /^Review and publish a rule$/ })).toBeTruthy());
    expect(screen.getAllByLabelText("Sign-in credential (held in memory)")).toHaveLength(1);
  });
});
