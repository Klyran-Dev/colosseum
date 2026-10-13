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

  it("localizes the recovery state without exposing exception details", () => {
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    render(
      <WorkspaceErrorBoundary workspace="governance" locale="de-DE">
        <BrokenWorkspace />
      </WorkspaceErrorBoundary>,
    );

    expect(screen.getByRole("heading", { name: "Dieser Arbeitsbereich konnte nicht geladen werden" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Anwendung neu laden" })).toBeTruthy();
  });

  it("self-recovers once when a stale lazy chunk is no longer available", () => {
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    const reload = vi.spyOn(window.location, "reload").mockImplementation(() => undefined);
    render(
      <WorkspaceErrorBoundary workspace="governance">
        <BrokenWorkspace />
      </WorkspaceErrorBoundary>,
    );

    expect(reload).not.toHaveBeenCalled();

    cleanup();
    render(
      <WorkspaceErrorBoundary workspace="governance">
        <BrokenLazyWorkspace />
      </WorkspaceErrorBoundary>,
    );

    expect(reload).toHaveBeenCalledTimes(1);
    expect(window.sessionStorage.getItem("solana-tax-protocol:chunk-recovery")).toMatch(/^\d+$/);

    cleanup();
    render(
      <WorkspaceErrorBoundary workspace="governance">
        <BrokenLazyWorkspace />
      </WorkspaceErrorBoundary>,
    );
    expect(reload).toHaveBeenCalledTimes(1);
  });
});

class BrokenLazyWorkspace extends Component {
  render(): ReactNode {
    throw new Error("Failed to fetch dynamically imported module: /assets/GovernancePanel-old.js");
  }
}
