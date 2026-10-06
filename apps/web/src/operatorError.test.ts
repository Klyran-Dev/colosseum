import { describe, expect, it } from "vitest";
import { operatorErrorMessage, operatorFacingError } from "./operatorError";

describe("operator error presentation boundary", () => {
  it("uses the workspace fallback for an ordinary exception", () => {
    expect(operatorErrorMessage(new Error("provider-internal diagnostic"), "Payment could not be prepared.")).toBe("Payment could not be prepared.");
  });

  it("preserves explicitly authored operator copy", () => {
    expect(operatorErrorMessage(operatorFacingError("The quote has expired. Create a new quote."), "Payment could not be prepared.")).toBe("The quote has expired. Create a new quote.");
  });

  it("keeps an approved support reference while using the workspace fallback", () => {
    expect(operatorErrorMessage(operatorFacingError("technical response text", "01992f1b-4a70-7e30-8ea2-b6e37a9d9d13"), "Payment could not be prepared.")).toBe("Payment could not be prepared. (support reference: 01992f1b-4a70-7e30-8ea2-b6e37a9d9d13)");
  });

  it("does not trust non-error rejection values", () => {
    expect(operatorErrorMessage({ message: "untrusted detail" }, "The request could not be completed.")).toBe("The request could not be completed.");
  });
});
