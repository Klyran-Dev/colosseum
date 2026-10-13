// @vitest-environment happy-dom

import { cleanup, render, screen } from "@testing-library/react";
import { Component, ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import AdminLazyWorkspace from "./AdminLazyWorkspace";

class BrokenLazyPage extends Component {
  render(): ReactNode {
    throw new Error("Failed to fetch dynamically imported module: /assets/RuleGovernancePage-old.js");
  }
}

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("AdminLazyWorkspace", () => {
  it("contains a stale lazy chunk failure behind the reviewed recovery surface", () => {
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    render(<AdminLazyWorkspace workspace="tax rules" fallback={<p>Loading test workspace</p>}><BrokenLazyPage /></AdminLazyWorkspace>);

    expect(screen.getByRole("alert")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "This workspace could not be loaded" })).toBeTruthy();
    expect(screen.queryByText(/RuleGovernancePage-old\.js/u)).toBeNull();
  });
});
