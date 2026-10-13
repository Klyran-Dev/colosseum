// @vitest-environment happy-dom

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import AdminApp from "./AdminApp";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

beforeEach(() => {
  vi.stubGlobal("fetch", vi.fn(async () => new Response(JSON.stringify({ error: "admin service unavailable" }), {
    status: 503,
    headers: { "content-type": "application/json" },
  })));
});

describe("full React admin application", () => {
  it("does not expose the protected dashboard before authentication", async () => {
    render(<AdminApp initialPath="/admin" />);

    expect(await screen.findByRole("heading", { name: "Sign in to Tax operations" })).toBeTruthy();
    expect(screen.queryByRole("complementary", { name: "Admin navigation" })).toBeNull();
    expect(screen.queryByText("RBAC · ABAC · PBAC")).toBeNull();
    expect(document.querySelector(".legacy-shell")).toBeNull();
    expect(document.querySelector("[data-testid='admin-chart']")).toBeNull();
    expect(document.querySelector("[data-testid='admin-table']")).toBeNull();
  });

  it("renders sign-in and sign-up as typed admin routes without a client role selector", async () => {
    const { unmount } = render(<AdminApp initialPath="/auth/signin" />);
    expect(await screen.findByRole("heading", { name: "Sign in to Tax operations" })).toBeTruthy();
    expect(await screen.findByRole("link", { name: "Create an account" })).toBeTruthy();
    expect(screen.queryByLabelText("Role")).toBeNull();

    unmount();
    render(<AdminApp initialPath="/auth/signup" />);
    expect(await screen.findByRole("heading", { name: "Create your operator account" })).toBeTruthy();
    expect(await screen.findByRole("link", { name: "Back to sign in" })).toBeTruthy();
    expect(screen.queryByLabelText("Role")).toBeNull();
  });
});
