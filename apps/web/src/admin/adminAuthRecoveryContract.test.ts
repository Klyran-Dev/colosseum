import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const source = readFileSync(resolve(process.cwd(), "src/admin/AdminAuthPage.tsx"), "utf8");

describe("admin authentication recovery contract", () => {
  it("surfaces server-bounded OTP expiry and resend recovery", () => {
    expect(source).toContain("expires_at_unix");
    expect(source).toContain("retry_after_seconds");
    expect(source).toContain("Resend code");
    expect(source).toContain("Use a different email");
  });
});
