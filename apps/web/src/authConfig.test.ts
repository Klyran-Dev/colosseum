import { describe, expect, it } from "vitest";
import { browserAuthConfigFromEnv, parseBrowserAuthConfig } from "./authConfig";

describe("browser authentication configuration", () => {
  it("wires the explicitly selected email OTP method from the release environment", () => {
    expect(browserAuthConfigFromEnv({
      VITE_DEPLOYMENT_ENVIRONMENT: "staging",
      VITE_AUTH_METHOD: "email-otp",
      VITE_AUTH_SERVICE_BASE_URL: "https://auth.klyran.com",
    })).toEqual({
      mode: "email-otp",
      emailOtp: { baseUrl: "https://auth.klyran.com" },
      warnings: [],
    });
  });

  it("accepts a complete HTTPS OIDC public-client configuration", () => {
    expect(parseBrowserAuthConfig({
      VITE_DEPLOYMENT_ENVIRONMENT: "production",
      VITE_OIDC_ISSUER: "https://identity.example.test/",
      VITE_OIDC_CLIENT_ID: "tax-console",
      VITE_OIDC_REDIRECT_URI: "https://console.example.test/auth/callback",
      VITE_OIDC_SCOPE: "openid profile",
      VITE_OIDC_AUDIENCE: "tax-api",
      VITE_LOCAL_CREDENTIAL_ENTRY: "false",
    })).toEqual({
      mode: "oidc",
      oidc: {
        issuer: "https://identity.example.test/",
        clientId: "tax-console",
        redirectUri: "https://console.example.test/auth/callback",
        scope: "openid profile",
        audience: "tax-api",
      },
      warnings: [],
    });
  });

  it("allows synthetic credential entry only for an explicitly local build", () => {
    expect(parseBrowserAuthConfig({
      VITE_DEPLOYMENT_ENVIRONMENT: "local",
      VITE_LOCAL_CREDENTIAL_ENTRY: "true",
    })).toEqual({ mode: "local-credential", warnings: [] });
  });

  it("accepts the local email OTP issuer only with an explicit loopback service URL", () => {
    expect(parseBrowserAuthConfig({
      VITE_DEPLOYMENT_ENVIRONMENT: "local",
      VITE_LOCAL_UI_MODE: "email-otp",
      VITE_AUTH_SERVICE_BASE_URL: "http://localhost:18085/",
    })).toEqual({
      mode: "email-otp",
      emailOtp: { baseUrl: "http://localhost:18085" },
      warnings: [],
    });
  });

  it("accepts the bounded Compose auth-service hostname only in local email OTP mode", () => {
    expect(parseBrowserAuthConfig({
      VITE_DEPLOYMENT_ENVIRONMENT: "local",
      VITE_LOCAL_UI_MODE: "email-otp",
      VITE_AUTH_SERVICE_BASE_URL: "http://auth-service:8085",
    })).toEqual({
      mode: "email-otp",
      emailOtp: { baseUrl: "http://auth-service:8085" },
      warnings: [],
    });
  });

  it("fails closed on email OTP outside local development", () => {
    const config = parseBrowserAuthConfig({
      VITE_DEPLOYMENT_ENVIRONMENT: "staging",
      VITE_LOCAL_UI_MODE: "email-otp",
      VITE_AUTH_SERVICE_BASE_URL: "https://auth.example.test",
    });

    expect(config.mode).toBe("unconfigured");
    expect(config.warnings[0]).toContain("permitted only in a local build");
  });

  it("accepts production email OTP only when the release explicitly selects it", () => {
    expect(parseBrowserAuthConfig({
      VITE_DEPLOYMENT_ENVIRONMENT: "production",
      VITE_AUTH_METHOD: "email-otp",
      VITE_AUTH_SERVICE_BASE_URL: "https://auth.example.test/",
    })).toEqual({
      mode: "email-otp",
      emailOtp: { baseUrl: "https://auth.example.test" },
      warnings: [],
    });
  });

  it("rejects an explicit production email OTP method without HTTPS transport", () => {
    const config = parseBrowserAuthConfig({
      VITE_DEPLOYMENT_ENVIRONMENT: "production",
      VITE_AUTH_METHOD: "email-otp",
      VITE_AUTH_SERVICE_BASE_URL: "http://auth.example.test",
    });

    expect(config.mode).toBe("unconfigured");
    expect(config.warnings).toContain("The email OTP service URL must use HTTPS outside local development.");
  });

  it("does not allow an explicit email OTP release to also provide OIDC settings", () => {
    const config = parseBrowserAuthConfig({
      VITE_DEPLOYMENT_ENVIRONMENT: "production",
      VITE_AUTH_METHOD: "email-otp",
      VITE_AUTH_SERVICE_BASE_URL: "https://auth.example.test",
      VITE_OIDC_ISSUER: "https://identity.example.test",
    });

    expect(config.mode).toBe("unconfigured");
    expect(config.warnings).toEqual(["Email OTP sign-in cannot be combined with OIDC sign-in configuration."]);
  });

  it("rejects local credential entry outside local development", () => {
    const config = parseBrowserAuthConfig({
      VITE_DEPLOYMENT_ENVIRONMENT: "staging",
      VITE_LOCAL_CREDENTIAL_ENTRY: "true",
    });

    expect(config.mode).toBe("unconfigured");
    expect(config.warnings).toEqual(["Local credential entry is permitted only in a local build."]);
  });

  it("fails closed on partial or unsafe OIDC configuration", () => {
    const config = parseBrowserAuthConfig({
      VITE_DEPLOYMENT_ENVIRONMENT: "production",
      VITE_OIDC_ISSUER: "http://identity.example.test",
      VITE_OIDC_CLIENT_ID: "tax-console",
      VITE_OIDC_REDIRECT_URI: "http://console.example.test/auth/callback",
      VITE_OIDC_SCOPE: "profile",
    });

    expect(config.mode).toBe("unconfigured");
    expect(config.warnings).toEqual([
      "The OIDC scope must include openid.",
      "The OIDC issuer must use HTTPS outside local development.",
      "The OIDC redirect URI must use HTTPS outside local development.",
    ]);
  });

  it("rejects an issuer with query or fragment data before CSP binding", () => {
    const config = parseBrowserAuthConfig({
      VITE_DEPLOYMENT_ENVIRONMENT: "production",
      VITE_OIDC_ISSUER: "https://identity.example.test/?tenant=all",
      VITE_OIDC_CLIENT_ID: "tax-console",
      VITE_OIDC_REDIRECT_URI: "https://console.example.test/auth/callback",
      VITE_OIDC_SCOPE: "openid",
    });

    expect(config.mode).toBe("unconfigured");
    expect(config.warnings).toContain("The OIDC issuer must use HTTPS outside local development.");
  });

  it("does not silently choose a login mode when no configuration is supplied", () => {
    expect(parseBrowserAuthConfig({})).toEqual({
      mode: "unconfigured",
      warnings: ["No browser sign-in configuration was supplied by the release build."],
    });
  });
});
