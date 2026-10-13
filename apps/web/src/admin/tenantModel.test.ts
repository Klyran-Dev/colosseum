import { describe, expect, it } from "vitest";
import { setupProgressLabel, tenantSetupSummary, type TenantContext } from "./tenantModel";

const context = (overrides: Partial<TenantContext> = {}): TenantContext => ({
  schema: "auth-tenant-context/v1",
  tenantId: "0199d6f4-2b2a-7c31-8c6d-8b0b1b5c9014",
  displayName: "Klyran workspace",
  status: "active",
  readiness: {
    state: "in_progress",
    completedSteps: 1,
    totalSteps: 3,
    steps: [
      { id: "workspace_profile", label: "Name your workspace", status: "complete", actionPath: "/admin/settings" },
      { id: "settlement_route", label: "Verify a settlement route", status: "pending", actionPath: "/admin/settlements" },
      { id: "tax_policy", label: "Publish a tax policy", status: "pending", actionPath: "/admin/tax/rules" },
    ],
  },
  ...overrides,
});

describe("tenant readiness presentation", () => {
  it("summarizes server-owned setup progress without inventing readiness", () => {
    expect(tenantSetupSummary(context())).toEqual({ completed: 1, total: 3, state: "in_progress", label: "1 of 3 setup steps complete" });
    expect(setupProgressLabel("complete")).toBe("Ready");
    expect(setupProgressLabel("pending")).toBe("Next step");
  });

  it("keeps a degraded tenant context actionable and fail-closed", () => {
    const unavailable = context({ status: "suspended", readiness: { state: "blocked", completedSteps: 0, totalSteps: 3, steps: [] } });
    expect(tenantSetupSummary(unavailable)).toEqual({ completed: 0, total: 3, state: "blocked", label: "Setup is blocked" });
  });
});
