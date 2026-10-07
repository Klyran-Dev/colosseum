import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      "/api": "http://api:8080"
    }
  },
  build: {
    sourcemap: false,
    target: "es2022"
  },
  test: {
    exclude: ["e2e/**", "node_modules/**", "dist/**"],
    // Opt-in DOM capture for style verification (ADR-0429); inert unless the
    // variable is set by scripts/web_style_fixtures.sh.
    setupFiles: process.env.STYLE_FIXTURE_DIR ? ["./src/styleFixtureCapture.setup.ts"] : [],
    // The package ships extensionless ESM imports that Node cannot resolve
    // natively; inlining lets Vite resolve them (test/build-time only).
    server: { deps: { inline: ["@material/material-color-utilities"] } }
  }
});
