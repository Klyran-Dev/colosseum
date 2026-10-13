// @vitest-environment happy-dom

import { afterEach, describe, expect, it, vi } from "vitest";
import { fetchAdminSnapshot } from "./data";

afterEach(() => vi.unstubAllGlobals());

describe("admin data adapters", () => {
  it("maps the server capability snapshot and redacted evidence projection without inventing totals", async () => {
    vi.stubGlobal("fetch", vi.fn(async (input: RequestInfo | URL) => {
      const path = String(input);
      if (path.includes("authorization/capabilities")) return new Response(JSON.stringify({ schema: "authorization-capabilities/v1", snapshot_id: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9013", subject: "operator-1", tenant_id: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9014", expires_at_unix: 1_800_000_060, roles: ["administrator"], capabilities: { "evidence.audit.read": { decision: "allow", reason_code: "scope_and_role_satisfied" }, "tax.quote.create": { decision: "deny", reason_code: "role_missing" }, "tax.rule.govern": { decision: "indeterminate", reason_code: "resource_context_required" } } }), { status: 200, headers: { "content-type": "application/json" } });
      return new Response(JSON.stringify([{ id: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9015", calculation_id: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9016", actor_subject: "operator-1", purpose: "replay", outcome: "accepted", occurred_at: "2026-10-11T08:00:00Z" }]), { status: 200, headers: { "content-type": "application/json" } });
    }));

    const snapshot = await fetchAdminSnapshot({ accessToken: "token", email: "operator@example.com", expiresAtUnix: 1_800_000_060 });

    expect(snapshot.roles).toEqual(["administrator"]);
    expect(snapshot.metrics.find((metric) => metric.id === "allowed")?.value).toBe("1");
    expect(snapshot.metrics.find((metric) => metric.id === "denied")?.value).toBe("1");
    expect(snapshot.recentActions[0]).toMatchObject({ action: "replay", subject: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9016", status: "verified" });
    expect(snapshot.policySeries).toEqual([{ label: "10-11", value: 1 }]);
  });

  it("fails closed when a successful response is not the versioned authorization contract", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => new Response(JSON.stringify({ capabilities: {} }), { status: 200, headers: { "content-type": "application/json" } })));

    await expect(fetchAdminSnapshot({ accessToken: "token", email: "operator@example.com", expiresAtUnix: 1_800_000_060 })).rejects.toMatchObject({
      code: "invalid",
    });
  });
});
