import { describe, expect, it } from "vitest";
import { Buffer } from "buffer";
import vector from "../../../test-vectors/calculation-attestation-v1.json";
import { buildCalculationAttestationPayload, sha256Digest } from "./policy";

describe("calculation attestation compatibility boundary", () => {
  it("matches the Rust canonical payload and digest vector", async () => {
    const payload = buildCalculationAttestationPayload(vector.unsigned);
    expect(Buffer.from(payload).toString("base64")).toBe(vector.canonical_payload_base64);
    expect([...await sha256Digest(payload)]).toEqual(vector.payload_digest);
  });

  it("keeps atomic values out of JavaScript Number conversion", () => {
    expect(() => buildCalculationAttestationPayload({
      ...vector.unsigned,
      gross_atomic: "9007199254740993",
      tax_atomic: "1",
      net_atomic: "9007199254740992",
    })).not.toThrow();
    expect(() => buildCalculationAttestationPayload({
      ...vector.unsigned,
      gross_atomic: 9007199254740993,
    })).toThrow(/canonical unsigned/);
  });

  it("rejects digest-shaped and text-shaped drift before signing", () => {
    expect(() => buildCalculationAttestationPayload({
      ...vector.unsigned,
      merchant_id: [...vector.unsigned.merchant_id.slice(0, 31)],
    })).toThrow(/32 bytes/);
    expect(() => buildCalculationAttestationPayload({
      ...vector.unsigned,
      rule_set_id: "é".repeat(65),
    })).toThrow(/128 bytes/);
  });
});
