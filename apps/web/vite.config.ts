import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    port: 5173,
    proxy: {
      "/api": "http://api:8080"
    }
  },
  build: {
    sourcemap: false,
    target: "es2022",
    // The stylesheet uses color-mix(), container queries, :is() and linear()
    // easing. State the supported browsers so the CSS minifier never lowers or
    // drops them (ADR-0436): Chrome/Edge 113+, Firefox 113+, Safari 17.2+.
    cssTarget: ["chrome113", "edge113", "firefox113", "safari17.2"]
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
