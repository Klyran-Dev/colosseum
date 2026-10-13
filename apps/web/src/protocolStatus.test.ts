import { describe, expect, it } from "vitest";
import { formatProtocolStatus } from "./protocolStatus";

describe("protocol status presentation", () => {
  it("uses the locale catalog for known lifecycle values", () => {
    expect(formatProtocolStatus("submitted", "de-DE")).toBe("Übermittelt");
    expect(formatProtocolStatus("prepared_unsubmitted", "ar-EG")).toBe("مُعد — لم يُرسل");
  });

  it("does not expose an unknown backend value to operators", () => {
    expect(formatProtocolStatus("provider_internal_state", "ja-JP")).toBe("不明");
  });
});
