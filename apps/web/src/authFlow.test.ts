import { describe, expect, it } from "vitest";
import { authIntentFromPath, authPathForIntent, consumeAuthReturnPath, invitationIdFromSearch, isAuthPath, safeAuthNextPath, safeInvitationId, storeAuthReturnPath } from "./authFlow";

describe("dedicated authentication routes", () => {
  it("distinguishes sign-in and sign-up without accepting an external return URL", () => {
    expect(authIntentFromPath("/auth/signup")).toBe("signup");
    expect(authIntentFromPath("/auth/signin")).toBe("signin");
    expect(isAuthPath("/auth/signin")).toBe(true);
    expect(safeAuthNextPath("/operations/tax-quotes")).toBe("/operations/tax-quotes");
    expect(safeAuthNextPath("https://evil.example/steal")).toBe("/operations/tax-simulator");
    expect(safeAuthNextPath("//evil.example/steal")).toBe("/operations/tax-simulator");
    expect(safeAuthNextPath("/auth/signup")).toBe("/operations/tax-simulator");
    expect(authPathForIntent("signup", "/operations/tax-rules")).toBe("/auth/signup?next=%2Foperations%2Ftax-rules");
  });

  it("keeps only a bounded invitation reference on the sign-up route", () => {
    const invitationId = "0192e6ec-4d8f-7d2e-a40e-2c095f871405";
    expect(safeInvitationId(invitationId)).toBe(invitationId);
    expect(safeInvitationId("0192e6ec-4d8f-4d2e-a40e-2c095f871405")).toBeNull();
    expect(safeInvitationId("https://evil.example/invite")).toBeNull();
    expect(invitationIdFromSearch(`?invite=${invitationId}`)).toBe(invitationId);
    expect(invitationIdFromSearch("?invite=not-an-id")).toBeNull();
    expect(authPathForIntent("signup", "/rules/access-approvals", invitationId)).toBe(`/auth/signup?next=%2Frules%2Faccess-approvals&invite=${invitationId}`);
    expect(authPathForIntent("signin", "/rules/access-approvals", invitationId)).toBe("/auth/signin?next=%2Frules%2Faccess-approvals");
  });

  it("stores and consumes the OIDC return path exactly once", () => {
    const storage = new Map<string, string>();
    const session = {
      getItem: (key: string) => storage.get(key) ?? null,
      setItem: (key: string, value: string) => { storage.set(key, value); },
      removeItem: (key: string) => { storage.delete(key); },
    } as unknown as Storage;

    storeAuthReturnPath(session, "/rules/access-approvals");
    expect(consumeAuthReturnPath(session)).toBe("/rules/access-approvals");
    expect(consumeAuthReturnPath(session)).toBe("/operations/tax-simulator");
  });
});
