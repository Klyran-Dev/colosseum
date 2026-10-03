import { afterEach, describe, expect, it, vi } from "vitest";
import { createClientNonce, createClientUuid } from "./clientIdentity";

afterEach(() => vi.unstubAllGlobals());

describe("client idempotency identity", () => {
  it("uses a CSPRNG UUID fallback when randomUUID is unavailable in an insecure local context", () => {
    const bytes = Uint8Array.from({ length: 16 }, (_, index) => index * 17);
    vi.stubGlobal("crypto", {
      getRandomValues(target: Uint8Array) {
        target.set(bytes);
        return target;
      },
    });

    expect(createClientUuid()).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/);
  });

  it("fails closed when no cryptographically secure random source exists", () => {
    vi.stubGlobal("crypto", undefined);
    expect(() => createClientUuid()).toThrow("cryptographically secure random source");
  });

  it("creates a positive canonical decimal u64 nonce from secure random words", () => {
    vi.stubGlobal("crypto", {
      getRandomValues(target: Uint32Array) {
        target[0] = 1;
        target[1] = 1;
        return target;
      },
    });

    expect(createClientNonce()).toBe("4294967297");
  });

  it("fails closed when a secure nonce source is unavailable", () => {
    vi.stubGlobal("crypto", undefined);
    expect(() => createClientNonce()).toThrow("cryptographically secure random source");
  });
});
