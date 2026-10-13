import { describe, expect, it } from "vitest";
import { parseTenantContext } from "./tenantData";

describe("tenant context contract", () => {
  it("accepts only a bounded server-owned context", async () => {
    const payload = {
      schema: "auth-tenant-context/v1",
      tenant_id: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9014",
      display_name: "Klyran operations",
      status: "active",
      readiness: {
        state: "ready",
        completed_steps: 3,
        total_steps: 3,
        steps: [
          { id: "workspace_profile", label: "Name your workspace", status: "complete", action_path: "/admin/settings" },
          { id: "settlement_route", label: "Verify a settlement route", status: "complete", action_path: "/admin/settlements" },
          { id: "tax_policy", label: "Publish a tax policy", status: "complete", action_path: "/admin/tax/rules" },
        ],
      },
    };
    const result = parseTenantContext(payload);
    expect(result.displayName).toBe("Klyran operations");
    expect(result.readiness.state).toBe("ready");
  });

  it("rejects a context that would fabricate a missing setup step", async () => {
    const payload = {
      schema: "auth-tenant-context/v1",
      tenant_id: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9014",
      display_name: "Klyran operations",
      status: "active",
      readiness: { state: "ready", completed_steps: 3, total_steps: 3, steps: [] },
    };
    expect(() => parseTenantContext(payload)).toThrow(/setup steps/i);
  });
});
