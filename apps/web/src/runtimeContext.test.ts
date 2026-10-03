import { describe, expect, it } from "vitest";
import { parseRuntimeContext } from "./runtimeContext";

describe("browser runtime context", () => {
  it("maps only the explicitly supported deployment and Solana values", () => {
    expect(parseRuntimeContext({
      VITE_DEPLOYMENT_ENVIRONMENT: "staging",
      VITE_SOLANA_NETWORK: "devnet",
    })).toMatchObject({
      environment: { value: "staging", label: "Staging", configured: true },
      network: { value: "devnet", label: "Solana Devnet", configured: true },
      configured: true,
      warnings: [],
    });
  });

  it("does not invent a plausible environment or network when build context is absent", () => {
    const context = parseRuntimeContext({});

    expect(context.environment).toMatchObject({ value: null, label: "Not configured", configured: false });
    expect(context.network).toMatchObject({ value: null, label: "Not configured", configured: false });
    expect(context.configured).toBe(false);
    expect(context.warnings).toEqual([
      "The deployment environment was not supplied by the release build.",
      "The Solana network was not supplied by the release build.",
    ]);
  });

  it("rejects invalid values instead of normalizing them into a different context", () => {
    const context = parseRuntimeContext({
      VITE_DEPLOYMENT_ENVIRONMENT: "production-ish",
      VITE_SOLANA_NETWORK: "mainnet",
    });

    expect(context.environment).toMatchObject({ value: null, label: "Not configured", configured: false });
    expect(context.network).toMatchObject({ value: null, label: "Not configured", configured: false });
    expect(context.warnings).toEqual([
      "The deployment environment is invalid for this release build.",
      "The Solana network is invalid for this release build.",
    ]);
  });
});
