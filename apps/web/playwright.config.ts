import { defineConfig } from "@playwright/test";

const baseURL = process.env.WEB_E2E_BASE_URL ?? "http://127.0.0.1:8088";
const authBrowserSpec = process.env.WEB_E2E_SPEC;

export default defineConfig({
  testDir: "./e2e",
  // The shipped entrypoint is AdminApp; legacy workspace specs are retained
  // as historical fixtures but are not the live submission browser gate.
  testMatch: authBrowserSpec ?? "admin-browser.spec.ts",
  testIgnore: authBrowserSpec ? undefined : ["**/auth-journey.spec.ts", "**/initial-admin-journey.spec.ts"],
  timeout: 30_000,
  expect: { timeout: 5_000 },
  fullyParallel: false,
  forbidOnly: Boolean(process.env.CI),
  retries: 0,
  workers: 1,
  reporter: process.env.CI
    ? [["line"], ["json", { outputFile: "/workspace/artifacts/web-e2e/results.json" }]]
    : [["list"]],
  outputDir: "/workspace/artifacts/web-e2e",
  use: {
    baseURL,
    colorScheme: "dark",
    reducedMotion: "reduce",
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
    video: "retain-on-failure",
  },
});
