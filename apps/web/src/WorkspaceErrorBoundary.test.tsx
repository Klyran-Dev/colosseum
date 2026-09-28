// @vitest-environment happy-dom

import { cleanup, render, screen } from "@testing-library/react";
import { Component, ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import WorkspaceErrorBoundary from "./WorkspaceErrorBoundary";

class BrokenWorkspace extends Component<{ children?: ReactNode }> {
  render(): ReactNode {
    throw new Error("secret-provider-key-must-not-render");
  }
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("workspace error containment", () => {
  it("renders an accessible recovery state without exposing raw exception details", () => {
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    render(
      <WorkspaceErrorBoundary workspace="governance">
        <BrokenWorkspace />
      </WorkspaceErrorBoundary>,
    );

    expect(screen.getByRole("alert")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "This workspace could not be loaded" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Reload application" })).toBeTruthy();
    expect(screen.queryByText("secret-provider-key-must-not-render")).toBeNull();
  });

  it("recovers when the application remounts it for a new workspace", () => {
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    const { rerender } = render(
      <WorkspaceErrorBoundary key="governance" workspace="governance">
        <BrokenWorkspace />
      </WorkspaceErrorBoundary>,
    );

    expect(screen.getByRole("alert")).toBeTruthy();

    rerender(
      <WorkspaceErrorBoundary key="checkout" workspace="checkout">
        <p>Checkout workspace loaded</p>
      </WorkspaceErrorBoundary>,
    );

    expect(screen.getByText("Checkout workspace loaded")).toBeTruthy();
    expect(screen.queryByRole("alert")).toBeNull();
  });
});
