// @vitest-environment node

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const stylesheet = readFileSync(resolve(process.cwd(), "src/styles.css"), "utf8");
const documentSource = readFileSync(resolve(process.cwd(), "index.html"), "utf8");

describe("Material 3 Expressive shell contract", () => {
  it("uses the v3 Material baseline role matrix and explicit semantic state containers", () => {
    expect(stylesheet).toContain("Material 3 Expressive theme family: Material Baseline / seed #6750A4 / TonalSpot / role matrix v3");
    expect(stylesheet).toContain("--app-success-container: var(--md-sys-color-primary-container);");
    expect(stylesheet).toContain("--app-attention-container: var(--md-sys-color-warning-container);");
    expect(stylesheet).toContain("--app-blocked-container: var(--md-sys-color-error-container);");
    expect(stylesheet).toContain("--app-supporting-container: var(--md-sys-color-tertiary-container);");
  });

  it("keeps browser chrome on the approved M3 background roles", () => {
    expect(documentSource).toContain('name="theme-color" content="#141218" data-theme-color="dark"');
    expect(documentSource).toContain('name="theme-color" content="#FDF7FF" data-theme-color="light"');
    expect(documentSource).not.toContain("#08111f");
    expect(documentSource).not.toContain("#0F1014");
  });

  it("keeps the final shell layer role-based and free of decorative gradients", () => {
    const finalLayer = stylesheet.slice(stylesheet.lastIndexOf("/* Material 3 Expressive final precedence layer."));
    expect(finalLayer).not.toMatch(/#[0-9a-f]{3,8}|rgba?\(/i);
    expect(finalLayer).not.toMatch(/gradient\(/i);
    expect(finalLayer).toContain("background: var(--app-secondary-container);");
    expect(finalLayer).toContain("background: var(--app-surface-container-low);");
    expect(finalLayer).toContain("background: var(--app-warning-container);");
  });

  it("keeps the expressive shell quiet through tonal layering instead of stacked chrome", () => {
    const finalLayer = stylesheet.slice(stylesheet.lastIndexOf("/* Material 3 Expressive final precedence layer."));
    expect(finalLayer).toContain(".product-shell .product-topbar { background: var(--app-surface-container); }");
    expect(finalLayer).toContain(".product-shell .product-content { width: min(1200px, calc(100% - 3rem)); }");
    expect(finalLayer).toContain(".product-shell .system-status > summary { background: var(--app-surface-container-low); }");
    expect(finalLayer).not.toContain("backdrop-filter");
  });

  it("reserves filled state roles for actual state and keeps neutral badges quiet", () => {
    const finalLayer = stylesheet.slice(stylesheet.lastIndexOf("/* Material 3 Expressive final precedence layer."));
    expect(finalLayer).toContain(".product-shell .status-pill { color: var(--app-on-variant); background: var(--app-surface-container); }");
    expect(finalLayer).toContain(".product-shell .status-pill-success { color: var(--app-on-success-container); background: var(--app-success-container); }");
    expect(finalLayer).toContain(".product-shell .status-pill-danger { color: var(--app-on-blocked-container); background: var(--app-blocked-container); }");
  });

  it("keeps workflow risk taxonomy neutral instead of presenting it as an incident", () => {
    const finalLayer = stylesheet.slice(stylesheet.lastIndexOf("/* Material 3 Expressive final precedence layer."));
    expect(finalLayer).toContain(".product-shell .risk-read, .product-shell .risk-write, .product-shell .risk-financial, .product-shell .risk-governance { color: var(--app-on-variant); background: var(--app-surface-container); border-color: transparent; }");
    expect(finalLayer).not.toContain(".risk-financial { color: var(--app-on-blocked-container); background: var(--app-blocked-container); }");
    expect(finalLayer).not.toContain(".risk-governance { color: var(--app-on-supporting-container); background: var(--app-supporting-container); }");
  });

  it("keeps active navigation tonal and free of a second border emphasis", () => {
    const finalLayer = stylesheet.slice(stylesheet.lastIndexOf("/* Material 3 Expressive final precedence layer."));
  });

  it("does not retain styling for the removed duplicate sidebar account context", () => {
    expect(stylesheet).not.toMatch(/(?:^|[,{\s])\.sidebar-context(?:[\s,{])/m);
  });

  it("sizes workspace layouts from the available workspace, not the viewport (ADR-0421)", () => {
    const layer = stylesheet.slice(stylesheet.lastIndexOf("/* ADR-0421:"));
    expect(layer).toContain("container: workspace / inline-size;");
    for (const layout of ["workspace-grid", "checkout-layout", "governance-layout", "replay-layout", "vault-layout", "adjustment-layout"]) {
      expect(layer).toContain(`.product-shell .${layout}`);
    }
    expect(layer).toMatch(/@container workspace \(max-width: 56rem\)/);
    expect(layer).toContain("repeat(auto-fit, minmax(min(100%, 15rem), 1fr))");
    expect(layer).not.toMatch(/#[0-9a-f]{3,8}|rgba?\(|gradient\(|backdrop-filter/i);
  });

  it("keeps form controls from truncating their labels and placeholders", () => {
    const layer = stylesheet.slice(stylesheet.lastIndexOf("/* ADR-0421:"));
    expect(layer).toContain(".product-shell .form-grid > * { min-width: 0; }");
    expect(layer).toContain("text-overflow: ellipsis;");
    expect(layer).toContain(".product-shell .panel > .inline-warning + .route-evidence");
  });

  it("applies Expressive shape, spring motion, type roles and state layers to controls (ADR-0422)", () => {
    const layer = stylesheet.slice(stylesheet.lastIndexOf("/* ADR-0422:"));
    expect(layer).not.toMatch(/#[0-9a-f]{3,8}|rgba?\(|gradient\(|backdrop-filter/i);
    expect(layer).toContain("border-radius: var(--md-sys-shape-corner-full);");
    expect(layer).toContain("border-radius: var(--md-sys-shape-corner-medium);");
    expect(layer).toContain("var(--md-sys-motion-spring-fast-spatial-duration) var(--md-sys-motion-spring-fast-spatial-easing)");
    expect(layer).toContain("calc(var(--md-sys-state-hover-opacity) * 100%)");
    expect(layer).toContain("calc(var(--md-sys-state-pressed-opacity) * 100%)");
    expect(layer).toContain("calc(var(--md-sys-state-focus-opacity) * 100%)");
    expect(layer).toContain("var(--md-sys-typescale-headline-medium-size)");
    expect(layer).toContain("var(--md-sys-typescale-title-large-size)");
    expect(layer).toContain("var(--md-sys-typescale-body-medium-size)");
    expect(layer).toContain("background: var(--app-secondary-container);");
    expect(layer).toContain("border-radius: var(--md-sys-shape-corner-extra-small);");
  });

  it("keeps disabled controls explained by role colour, not opacity alone", () => {
    const layer = stylesheet.slice(stylesheet.lastIndexOf("/* ADR-0422:"));
    expect(layer).toContain("color: color-mix(in srgb, var(--app-on-surface) 38%, transparent);");
    expect(layer).toContain("background: color-mix(in srgb, var(--app-on-surface) 12%, transparent);");
  });

  describe("canonical navigation and panel skin (ADR-0423)", () => {
    const canonical = () => stylesheet.slice(stylesheet.lastIndexOf("/* ADR-0423:"));
    const outsideForcedColors = () => stylesheet.replace(/@media \(forced-colors: active\) \{[\s\S]*?\n\}\n/g, "");

    it("uses the M3 navigation drawer item: pill, secondary-container indicator, plain icon", () => {
      const layer = canonical();
      expect(layer).toContain(".product-shell .nav-item-active {\n  color: var(--app-on-secondary-container);\n  background: var(--app-secondary-container);");
      expect(layer).toContain("border-radius: var(--md-sys-shape-corner-full);");
      expect(layer).toContain("min-height: 3.5rem;");
      expect(layer).toContain("calc(var(--md-sys-state-hover-opacity) * 100%), transparent)");
      expect(layer).toContain(".product-shell .nav-item-icon,\n.product-shell .nav-item-active .nav-item-icon {\n  border-color: transparent;\n  color: inherit;\n  background: transparent;");
      expect(layer).toContain("var(--md-sys-typescale-title-small-size)");
      expect(layer).not.toMatch(/inset 3px|inset 4px|text-transform: uppercase/);
      expect(layer).not.toMatch(/#[0-9a-f]{3,8}|rgba?\(|gradient\(|backdrop-filter/i);
    });

    it("keeps navigation geometry owned by the compact-rail and mobile rules", () => {
      expect(canonical()).not.toMatch(/\.nav-item[^{]*\{[^}]*(grid-template-columns|padding)/);
    });

    it("uses tonal cards without borders or shadows", () => {
      const layer = canonical();
      expect(layer).toContain(".product-shell .panel,\n.product-shell .metric-card {\n  border-color: transparent;");
      expect(layer).toContain("background: var(--app-surface-container-low);\n  box-shadow: none;");
      expect(layer).toContain(".product-shell .panel { border-radius: var(--md-sys-shape-corner-extra-large); }");
      expect(layer).toContain(".product-shell .metric-card { border-radius: var(--md-sys-shape-corner-large); }");
      expect(layer).toContain("border-radius: var(--md-sys-shape-corner-medium);");
    });

    it("defines each navigation and panel skin exactly once", () => {
      const css = outsideForcedColors();
      const count = (pattern: RegExp) => (css.match(pattern) ?? []).length;
      expect(count(/^(?:\.product-shell )?\.nav-item-active \{/gm)).toBe(1);
      expect(count(/^(?:\.product-shell )?\.nav-item:hover/gm)).toBe(1);
      expect(count(/^(?:\.product-shell )?\.nav-item-icon \{/gm)).toBeLessThanOrEqual(2);
      expect(count(/^\.product-shell \.panel,\s*$/gm)).toBe(1);
      expect(count(/^\.product-shell \.app-sidebar \{/gm)).toBe(1);
    });

    it("keeps the forced-colors layer last so system colours win (ADR-0338)", () => {
      const forced = stylesheet.lastIndexOf("/* Forced-colors must remain the final cascade layer");
      expect(forced).toBeGreaterThan(stylesheet.lastIndexOf("/* ADR-0423:"));
      expect(forced).toBeGreaterThan(stylesheet.lastIndexOf("/* ADR-0422:"));
      expect(forced).toBeGreaterThan(stylesheet.lastIndexOf("/* ADR-0421:"));
      expect(stylesheet.slice(forced).match(/@media/g)).toHaveLength(1);
    });
  });

  it("does not apply content-button shape and padding to shell chrome controls (ADR-0422)", () => {
    const layer = stylesheet.slice(stylesheet.lastIndexOf("/* ADR-0422:"), stylesheet.lastIndexOf("/* ADR-0423:"));
    const buttonRules = layer.match(/^\.product-shell button:not\([^{]*\{/gm) ?? [];
    expect(buttonRules.length).toBeGreaterThan(0);
    for (const rule of buttonRules) {
      expect(rule, "chrome toggle must stay out of the content-button rules").toContain(":not(.navigation-compact-toggle)");
    }
  });
});
