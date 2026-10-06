import { describe, expect, it } from "vitest";
import { SUPPORTED_LOCALES, SupportedLocale } from "./formatters";
import { ProductWorkspace } from "./navigation";
import { UiAccessReasonCode, WorkspaceGroup } from "./authorization";
import { shellMessagesFor } from "./shellMessages";

const WORKSPACES: readonly ProductWorkspace[] = [
  "quote",
  "preview",
  "document-quote",
  "checkout",
  "document-checkout",
  "governance",
  "access-control",
  "vault-release",
  "vault-reconciliation",
  "revenue-streaming",
  "adjustments",
  "refund-claims",
  "refund-cancellations",
  "refund-executions",
  "replay-ops",
  "evidence-access",
];

const GROUPS: readonly WorkspaceGroup[] = ["Daily work", "Rules & access", "Money movement", "Records & issues"];
const ACCESS_REASON_CODES: readonly UiAccessReasonCode[] = [
  "sign_in_required",
  "server_evaluation_required",
  "server_denied",
  "missing_expiry",
  "clock_unavailable",
  "access_expired",
  "missing_capability_decision",
  "unknown_policy",
  "scope_and_role_satisfied",
  "scope_missing",
  "role_missing",
  "resource_context_required",
];

describe("versioned shell localization", () => {
  it("provides bounded non-empty navigation messages for every supported locale", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const messages = shellMessagesFor(locale);
      expect(Object.keys(messages.navigation).sort()).toEqual([...WORKSPACES].sort());
      for (const workspace of WORKSPACES) {
        expect(messages.navigation[workspace].label.length).toBeGreaterThan(0);
        expect(messages.navigation[workspace].description.length).toBeGreaterThan(0);
        const status = messages.workspaceStatus(messages.navigation[workspace].label);
        expect(status).toContain(messages.navigation[workspace].label);
        expect(status.length).toBeGreaterThan(messages.navigation[workspace].label.length);
      }
      for (const group of GROUPS) expect(messages.groups[group].length).toBeGreaterThan(0);
      for (const risk of ["read", "write", "financial", "governance"] as const) {
        expect(messages.riskLabels[risk].length).toBeGreaterThan(0);
      }
      for (const status of ["available", "unavailable", "needsCheck", "signInToCheck", "unknownPolicy"] as const) {
        expect(messages.accessStatus[status].length).toBeGreaterThan(0);
        expect(messages.accessStatus[status].length).toBeLessThanOrEqual(160);
      }
      for (const reasonCode of ACCESS_REASON_CODES) {
        expect(messages.accessReason(reasonCode).length).toBeGreaterThan(0);
        expect(messages.accessReason(reasonCode).length).toBeLessThanOrEqual(180);
      }
      expect(messages.chrome.openNavigation.length).toBeGreaterThan(0);
      expect(messages.chrome.displaySettings.length).toBeGreaterThan(0);
      expect(messages.chrome.dateTimeHelp("UTC")).toContain("UTC");
      expect(messages.session.signIn.length).toBeGreaterThan(0);
      expect(messages.session.signOut.length).toBeGreaterThan(0);
    }
  });

  it("keeps the English catalog as the deterministic fallback", () => {
    const english = shellMessagesFor("en-US");
    const unknown = shellMessagesFor("xx-XX" as SupportedLocale);
    expect(unknown.navigation.quote).toEqual(english.navigation.quote);
    expect(unknown.groups["Daily work"]).toBe(english.groups["Daily work"]);
    expect(unknown.chrome.displaySettings).toBe(english.chrome.displaySettings);
    expect(unknown.session.signIn).toBe(english.session.signIn);
  });

  it("does not translate protocol or authorization identifiers", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const messages = shellMessagesFor(locale);
      expect(messages.navigation.quote.label).not.toContain("tax:quote");
      expect(messages.navigation.quote.description).not.toContain("019");
    }
  });
});
