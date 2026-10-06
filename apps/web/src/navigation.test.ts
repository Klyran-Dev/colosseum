import { describe, expect, it } from "vitest";
import {
  allWorkspacePaths,
  isAuthenticationCallback,
  pathForWorkspace,
  workspaceFromPath,
} from "./navigation";

describe("static SPA workspace navigation", () => {
  it("maps every workspace to one stable path and back", () => {
    const paths = allWorkspacePaths();
    expect(paths).toHaveLength(16);
    expect(new Set(paths).size).toBe(paths.length);
    expect(new Set(paths.map((path) => workspaceFromPath(path)))).toEqual(new Set([
      "quote",
      "preview",
      "document-quote",
      "checkout",
      "document-checkout",
      "governance",
      "access-control",
      "evidence-access",
      "replay-ops",
      "vault-release",
      "vault-reconciliation",
      "revenue-streaming",
      "adjustments",
      "refund-claims",
      "refund-cancellations",
      "refund-executions",
    ]));
  });

  it("canonicalizes root, trailing slashes, and unknown paths to the quote workspace", () => {
    expect(workspaceFromPath("/")).toBe("quote");
    expect(workspaceFromPath("/unknown")).toBe("quote");
    expect(workspaceFromPath(`${pathForWorkspace("checkout")}/`)).toBe("checkout");
  });

  it("never creates a URL from sensitive or caller-supplied values", () => {
    expect(pathForWorkspace("refund-executions")).not.toMatch(/token|amount|tenant|digest|signature/i);
  });

  it("preserves OIDC callback parameters until authentication consumes them", () => {
    expect(isAuthenticationCallback("?code=authorization-code&state=csrf-state")).toBe(true);
    expect(isAuthenticationCallback("?error=access_denied&state=csrf-state")).toBe(true);
    expect(isAuthenticationCallback("?state=csrf-state")).toBe(false);
    expect(isAuthenticationCallback("?workspace=quote")).toBe(false);
  });
});
