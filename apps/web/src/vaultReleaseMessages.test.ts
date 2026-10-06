import { describe, expect, it } from "vitest";
import { SUPPORTED_LOCALES } from "./formatters";
import { vaultReleaseMessagesFor } from "./vaultReleaseMessages";

describe("vault release localization error catalog", () => {
  it("provides the complete bounded error catalog for every supported locale", () => {
    for (const locale of SUPPORTED_LOCALES) {
      const messages = vaultReleaseMessagesFor(locale);
      expect(messages.errors.authorizationRequired).toBeTruthy();
      expect(messages.errors.routeRequestFailed(422)).toContain("422");
      expect(messages.errors.claimProposalFailed).toBeTruthy();
      expect(messages.errors.claimLoaded).toBeTruthy();
      expect(messages.errors.transitionSaved).toBeTruthy();
    }
    expect(vaultReleaseMessagesFor("xx-XX").errors.claimProposalFailed)
      .toBe(vaultReleaseMessagesFor("en-US").errors.claimProposalFailed);
  });

  it("bounds untrusted transport status before it reaches operator copy", () => {
    expect(vaultReleaseMessagesFor("en-US").errors.routeRequestFailed(700))
      .toContain("unknown status");
  });

  it("does not put protocol or financial values in localized copy", () => {
    const renderedCopy = JSON.stringify(vaultReleaseMessagesFor("de-DE"));
    expect(renderedCopy).not.toMatch(/01992f1b|Tokenkeg|250000|111111111111/);
  });
});
