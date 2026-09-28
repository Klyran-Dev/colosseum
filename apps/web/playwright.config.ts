import { defineConfig } from "@playwright/test";

const baseURL = process.env.WEB_E2E_BASE_URL ?? "http://127.0.0.1:8088";

export default defineConfig({
  testDir: "./e2e",
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
