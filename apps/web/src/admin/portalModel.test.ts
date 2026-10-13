import { describe, expect, it } from "vitest";
import { ACCESS_POSTURE_ITEMS, NAV_ITEMS, portalIdentityForRoles, portalModeForRoles, visibleNavigationForSnapshot } from "./model";

describe("role-aware tenant portal model", () => {
  it("distinguishes tenant administrators from merchant operators", () => {
    expect(portalModeForRoles(["administrator"])).toBe("tenant-administrator");
    expect(portalModeForRoles(["merchant_operator"])).toBe("merchant-operations");
    expect(portalModeForRoles(["merchant_operator", "administrator"])).toBe("tenant-administrator");
    expect(portalIdentityForRoles(["administrator"]).label).toBe("Tenant administrator");
    expect(portalIdentityForRoles(["merchant_operator"]).label).toBe("Merchant operations");
  });

  it("fails closed for unknown roles instead of treating them as administrators", () => {
    expect(portalModeForRoles(["platform_admin"])).toBe("tenant-member");
    expect(portalIdentityForRoles(["platform_admin"]).label).toBe("Tenant member");
  });

  it("filters denied workspaces while keeping settings and the current route recoverable", () => {
    const snapshot = {
      capabilities: [
        { id: "tax.quote.create", decision: "allow" as const },
        { id: "tax.rule.govern", decision: "deny" as const },
        { id: "access.control", decision: "deny" as const },
      ],
      expiresAtUnix: 200,
    };
    const visible = visibleNavigationForSnapshot(snapshot, "rules", 100).map((item) => item.section);
    expect(visible).toEqual(expect.arrayContaining(["overview", "quotes", "settings", "rules"]));
    expect(visible).not.toContain("access");
    expect(NAV_ITEMS.some((item) => item.section === "settings")).toBe(true);
  });

  it("does not expose protected navigation while the snapshot is unavailable", () => {
    const visible = visibleNavigationForSnapshot(null).map((item) => item.section);
    expect(visible).toEqual(["overview", "settings"]);
  });

  it("fails closed when the server snapshot has expired", () => {
    const visible = visibleNavigationForSnapshot({
      expiresAtUnix: 100,
      capabilities: [{ id: "tax.quote.create", decision: "allow" }],
    }, undefined, 100).map((item) => item.section);
    expect(visible).toEqual(["overview", "settings"]);
  });

  it("keeps technical authorization terminology out of the primary workspace copy", () => {
    const copy = ACCESS_POSTURE_ITEMS.map((item) => `${item.label} ${item.description}`).join(" ");
    expect(copy).not.toMatch(/\b(?:RBAC|ABAC|PBAC)\b/);
    expect(ACCESS_POSTURE_ITEMS).toHaveLength(3);
  });
});
