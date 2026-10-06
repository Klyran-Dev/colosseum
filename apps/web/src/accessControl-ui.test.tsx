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
    expect(screen.getByText("Roles")).toBeTruthy();
    expect(screen.getByText("Permissions")).toBeTruthy();
    expect(screen.getByText("Conditions")).toBeTruthy();
    expect(screen.getByText("Access checked when you act")).toBeTruthy();
    expect(screen.getByText(/Safe by default/)).toBeTruthy();
    expect(screen.getByText(/RBAC/)).toBeTruthy();
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
