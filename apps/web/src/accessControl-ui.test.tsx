// @vitest-environment happy-dom

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import AccessControlPanel from "./AccessControlPanel";
import { AuthorizationSnapshot } from "./authorization";

afterEach(() => cleanup());

describe("access control workspace", () => {
  it("explains access in plain language while retaining administrator details", () => {
    const snapshot: AuthorizationSnapshot = {
      state: "server-allowed",
      subject: "operator-1",
      tenantId: "01900000-0000-7000-8000-000000000001",
      roles: [],
      scopes: [],
      attributes: {},
      capabilities: {},
    };
    render(<AccessControlPanel accessToken="opaque-token" authorizationSnapshot={snapshot} />);
    expect(screen.getByRole("heading", { name: "Who can do what" })).toBeTruthy();
    expect(screen.getAllByText("Roles").length).toBeGreaterThan(0);
    expect(screen.getByText("Permissions")).toBeTruthy();
    expect(screen.getByText("Conditions")).toBeTruthy();
    expect(screen.getAllByText("Access checked when you act").length).toBeGreaterThan(0);
    expect(screen.getByText(/Safe by default/)).toBeTruthy();
    expect(screen.getByText(/RBAC/)).toBeTruthy();
  });

  it("summarizes only the server-confirmed responsibilities and capability decisions", () => {
    const snapshot: AuthorizationSnapshot = {
      state: "server-allowed",
      subject: "moderator-1",
      email: "moderator@example.com",
      tenantId: "01900000-0000-7000-8000-000000000001",
      roles: ["merchant_operator", "moderator"],
      scopes: [],
      attributes: {},
      capabilities: {
        "tax.quote.create": { kind: "allow", reasonCode: "scope_and_role_satisfied", reason: "confirmed" },
        "access.control": { kind: "deny", reasonCode: "role_missing", reason: "denied" },
        "tax.preview.run": { kind: "indeterminate", reasonCode: "resource_context_required", reason: "context" },
      },
    };
    render(<AccessControlPanel accessToken="opaque-token" authorizationSnapshot={snapshot} />);
    expect(screen.getByRole("region", { name: "Your responsibilities" })).toBeTruthy();
    expect(screen.getByText("Operator")).toBeTruthy();
    expect(screen.getByText("Moderator")).toBeTruthy();
    expect(screen.getByText("1 available")).toBeTruthy();
    expect(screen.getByText("1 needs context")).toBeTruthy();
    expect(screen.getByText("1 not available")).toBeTruthy();
  });

  it("does not turn an incomplete or anonymous snapshot into fabricated zeroes", () => {
    const { rerender } = render(<AccessControlPanel accessToken="" />);
    expect(screen.getByRole("region", { name: "Your responsibilities" }).textContent).toContain("Sign in to check");
    expect(screen.queryByText("0 available")).toBeNull();
    rerender(<AccessControlPanel accessToken="opaque-token" authorizationSnapshot={{
      state: "server-allowed", subject: "operator", tenantId: "tenant", roles: [], scopes: [], attributes: {},
    }} authorizationError="The access check failed" />);
    expect(screen.getByRole("region", { name: "Your responsibilities" }).textContent).toContain("Access check unavailable");
    expect(screen.queryByText("0 available")).toBeNull();
  });

  it("shows an explicit unauthenticated state", () => {
    render(<AccessControlPanel accessToken="" />);
    expect(screen.getByText("Sign in needed")).toBeTruthy();
    expect(screen.getByText("Waiting for sign-in")).toBeTruthy();
  });

  it("localizes authorization explanations without changing policy identity", () => {
    render(<AccessControlPanel accessToken="" locale="de-DE" />);
    expect(screen.getByRole("heading", { name: "Wer darf was?" })).toBeTruthy();
    expect(screen.getByText("Regeln & Zugriff")).toBeTruthy();
    expect(screen.getAllByText("Steuerangebot erstellen").length).toBeGreaterThan(0);
    expect(screen.queryByText("tax.quote.create")).toBeNull();
  });
});
