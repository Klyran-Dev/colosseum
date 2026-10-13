// @vitest-environment happy-dom
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import RoleManagementPanel from "./RoleManagementPanel";

const allowedSnapshot = {
  state: "server-allowed" as const,
  subject: "admin",
  tenantId: "tenant",
  roles: ["administrator"],
  scopes: ["identity:role:manage"],
  attributes: {},
  expiresAtUnix: Math.floor(Date.now() / 1000) + 60,
  capabilities: { "identity.role.manage": { kind: "allow" as const, reasonCode: "scope_and_role_satisfied" as const, reason: "confirmed" } },
};

const directory = (roles: string[] = []) => ({
  schema: "auth-user-directory/v1",
  users: [{ id: "018f0f0f-0000-7000-8000-000000000001", email: "operator@example.com", status: "active", created_at_unix: 1, last_verified_at_unix: 2, roles }],
});

const invitations = (status = "pending") => ({
  schema: "auth-invitation-directory/v1",
  invitations: [{
    id: "018f0f0f-0000-7000-8000-000000000004",
    email: "moderator@example.com",
    role: "moderator",
    status,
    created_at_unix: 10,
    expires_at_unix: 20,
    accepted_at_unix: null,
    revoked_at_unix: null,
  }],
});

const roleCatalog = () => ({
  schema: "auth-role-catalog/v1",
  roles: [
    { id: "moderator", label: "Moderator", description: "Bounded review responsibility.", assignable: true },
    { id: "tax_rule_author", label: "Tax rule author", description: "Create and simulate governed tax-rule drafts.", assignable: true },
    { id: "vault_release_proposer", label: "Treasury release proposer", description: "Prepare a bounded treasury release for separate approval.", assignable: true },
  ],
});

afterEach(() => { cleanup(); vi.restoreAllMocks(); });

describe("role management affordance", () => {
  it("does not call the identity directory for a denied principal", () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch");
    render(<RoleManagementPanel apiBase="http://auth" accessToken="opaque" authorizationSnapshot={{ ...allowedSnapshot, capabilities: { "identity.role.manage": { kind: "deny", reasonCode: "role_missing", reason: "denied" } } }} />);
    expect(screen.getByText("Administrator approval required")).toBeTruthy();
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it("loads the tenant directory and records moderator assignment", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(new Response(JSON.stringify(directory()), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ schema: "auth-invitation-directory/v1", invitations: [] }), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(roleCatalog()), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ schema: "auth-role-mutation/v1", mutation_id: "018f0f0f-0000-7000-8000-000000000002" }), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(directory(["merchant_operator", "moderator"])), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ schema: "auth-invitation-directory/v1", invitations: [] }), { status: 200 }));
    render(<RoleManagementPanel apiBase="http://auth" accessToken="opaque" authorizationSnapshot={allowedSnapshot} />);
    expect(await screen.findByText("operator@example.com")).toBeTruthy();
    expect(screen.getByText(/assign the bounded responsibilities published by the service catalog/i)).toBeTruthy();
    expect(screen.getByRole("region", { name: "Tenant identity summary" })).toBeTruthy();
    expect(screen.getByText("Active people", { exact: true })).toBeTruthy();
    expect(screen.getByText("1", { selector: "strong" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Assign moderator" }));
    await waitFor(() => expect(fetchSpy).toHaveBeenCalledTimes(5));
    const mutationRequest = fetchSpy.mock.calls[3]?.[1] as RequestInit;
    expect(mutationRequest.method).toBe("POST");
    expect((mutationRequest.headers as Record<string, string>)["idempotency-key"]).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-7/iu);
    expect(await screen.findByText("Merchant operator", { selector: ".decision-chip" })).toBeTruthy();
    expect(await screen.findByText("Moderator", { selector: ".decision-chip" })).toBeTruthy();
    expect(await screen.findByText("Moderator responsibility assigned to operator@example.com.")).toBeTruthy();
  });

  it("uses the server role catalog for explicit domain responsibility assignment", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(new Response(JSON.stringify(directory()), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ schema: "auth-invitation-directory/v1", invitations: [] }), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(roleCatalog()), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ schema: "auth-role-mutation/v1", mutation_id: "018f0f0f-0000-7000-8000-000000000005" }), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(directory(["merchant_operator", "tax_rule_author"])), { status: 200 }));
    render(<RoleManagementPanel apiBase="http://auth" accessToken="opaque" authorizationSnapshot={allowedSnapshot} />);
    await screen.findByRole("option", { name: "Tax rule author" });
    fireEvent.change(screen.getByRole("combobox", { name: "Responsibility for operator@example.com" }), { target: { value: "tax_rule_author" } });
    fireEvent.click(screen.getByRole("button", { name: "Assign tax rule author to operator@example.com" }));
    await waitFor(() => expect(fetchSpy).toHaveBeenCalledTimes(5));
    expect(fetchSpy.mock.calls[3]?.[0]).toContain("/roles/tax_rule_author");
    expect(await screen.findByText("Tax rule author responsibility assigned to operator@example.com.")).toBeTruthy();
  });

  it("keeps the tenant summary unknown while server projections are loading", () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch").mockImplementation(() => new Promise(() => undefined));
    render(<RoleManagementPanel apiBase="http://auth" accessToken="opaque" authorizationSnapshot={allowedSnapshot} />);
    expect(screen.getByRole("region", { name: "Tenant identity summary" })).toBeTruthy();
    expect(screen.getAllByText("—", { selector: "strong" })).toHaveLength(3);
    expect(fetchSpy).toHaveBeenCalledTimes(3);
  });

  it("creates a tenant invitation through the identity service", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(new Response(JSON.stringify(directory()), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ schema: "auth-invitation-directory/v1", invitations: [] }), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(roleCatalog()), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ schema: "auth-invitation/v1", invitation_id: "018f0f0f-0000-7000-8000-000000000003" }), { status: 202 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ schema: "auth-invitation-directory/v1", invitations: [] }), { status: 200 }));
    render(<RoleManagementPanel apiBase="http://auth" accessToken="opaque" authorizationSnapshot={allowedSnapshot} />);
    await screen.findByText("operator@example.com");
    fireEvent.change(screen.getByRole("textbox", { name: "Invite by email" }), { target: { value: "new@example.com" } });
    fireEvent.click(screen.getByRole("button", { name: "Create invitation" }));
    await waitFor(() => expect(fetchSpy).toHaveBeenCalledTimes(5));
    const invitationRequest = fetchSpy.mock.calls[3]?.[1] as RequestInit;
    expect(invitationRequest.method).toBe("POST");
    expect(JSON.parse(String(invitationRequest.body))).toEqual({ email: "new@example.com", role: "merchant_operator" });
    expect((invitationRequest.headers as Record<string, string>)["idempotency-key"]).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-7/iu);
    expect(await screen.findByText(/Invitation created for new@example.com\. They can use the invitation link/)).toBeTruthy();
  });

  it("shows pending invitations and revokes them through the idempotent endpoint", async () => {
    const fetchSpy = vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(new Response(JSON.stringify(directory()), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(invitations()), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(roleCatalog()), { status: 200 }))
      .mockResolvedValueOnce(new Response(JSON.stringify({ schema: "auth-invitation/v1", invitation_id: "018f0f0f-0000-7000-8000-000000000004", status: "revoked" }), { status: 202 }))
      .mockResolvedValueOnce(new Response(JSON.stringify(invitations("revoked")), { status: 200 }));
    render(<RoleManagementPanel apiBase="http://auth" accessToken="opaque" authorizationSnapshot={allowedSnapshot} />);
    expect(await screen.findByText("moderator@example.com")).toBeTruthy();
    expect(await screen.findByText("Pending", { selector: ".status-pill" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Revoke invitation for moderator@example.com" }));
    await waitFor(() => expect(fetchSpy).toHaveBeenCalledTimes(5));
    const revokeRequest = fetchSpy.mock.calls[3]?.[1] as RequestInit;
    expect(revokeRequest.method).toBe("DELETE");
    expect((revokeRequest.headers as Record<string, string>)["idempotency-key"]).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-7/iu);
    expect(await screen.findByText("Revoked", { selector: ".status-pill" })).toBeTruthy();
  });
});

