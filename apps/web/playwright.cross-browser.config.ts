import { defineConfig, devices } from "@playwright/test";
import base from "./playwright.config";

/**
 * Cross-engine smoke run (ADR-0439): the product in real Firefox and WebKit, the
 * engines behind the stated browser baseline. Run with scripts/web_cross_browser_check.sh.
 */
export default defineConfig({
  ...base,
  testMatch: /cross-browser\.spec\.ts/,
  projects: [
    { name: "firefox", use: { ...devices["Desktop Firefox"] } },
    { name: "webkit", use: { ...devices["Desktop Safari"] } },
  ],
});
