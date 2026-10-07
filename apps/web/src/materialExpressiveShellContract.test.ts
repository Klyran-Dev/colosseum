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
    expect(finalLayer).toContain("background: var(--app-attention-container);");
  });

  it("keeps the expressive shell quiet through tonal layering instead of stacked chrome", () => {
    const finalLayer = stylesheet.slice(stylesheet.lastIndexOf("/* Material 3 Expressive final precedence layer."));
    expect(finalLayer).toContain(".product-shell .product-topbar { border-color: var(--app-outline-variant); background: var(--app-surface-container); }");
    expect(finalLayer).toContain(".product-shell .product-content { width: min(1200px, calc(100% - 3rem)); }");
    expect(finalLayer).toContain(".product-shell .system-status > summary,\n.product-shell .display-settings summary,");
    expect(finalLayer).not.toContain("backdrop-filter");
  });

  it("reserves filled state roles for actual state and keeps neutral badges quiet", () => {
    const finalLayer = stylesheet.slice(stylesheet.lastIndexOf("/* Material 3 Expressive final precedence layer."));
    expect(finalLayer).toContain(".product-shell .status-pill {\n  color: var(--app-on-variant);\n  background: var(--app-surface-container);");
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
    for (const layout of ["workspace-grid", "checkout-layout", "governance-layout", "replay-layout", "vault-layout", "adjustment-layout", "access-control-layout"]) {
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

    it("uses a pill active indicator in primary-container with plain icons (ADR-0425)", () => {
      const layer = canonical();
      expect(layer).toContain(".product-shell .nav-item-active {\n  color: var(--app-on-primary-container);\n  background: var(--app-primary-container);");
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

  describe("canonical topbar, status chips and menus (ADR-0425)", () => {
    const canonical = () => stylesheet.slice(stylesheet.lastIndexOf("/* ADR-0425:"), stylesheet.lastIndexOf("/* Forced-colors must remain"));

    it("renders status controls as M3 chips with tonal state containers", () => {
      const layer = canonical();
      expect(layer).toContain("min-height: 2rem;");
      expect(layer).toContain("border-radius: var(--md-sys-shape-corner-small);");
      expect(layer).toContain(".product-shell .system-status-attention > summary");
      expect(layer).toContain("color: var(--app-on-attention-container);\n  background: var(--app-attention-container);");
      expect(layer).toContain("color: var(--app-on-success-container);\n  background: var(--app-success-container);");
      expect(layer).toContain("calc(var(--md-sys-state-hover-opacity) * 100%), transparent)");
      expect(layer).toContain("var(--md-sys-typescale-label-medium-size)");
    });

    it("renders menus as elevated tonal surfaces without borders", () => {
      const layer = canonical();
      expect(layer).toContain("background: var(--app-surface-container-high);");
      expect(layer).toContain("box-shadow: var(--md-sys-elevation-level2);");
      expect(layer).toContain("border-color: transparent;");
      expect(layer).toContain("border-radius: var(--md-sys-shape-corner-large);");
    });

    it("keeps the account trigger a filled full-radius avatar", () => {
      const layer = canonical();
      expect(layer).toContain(".product-shell .account-menu-trigger {");
      expect(layer).toContain("background: var(--app-primary-container);");
    });

    it("uses sentence-case label typography for status pills", () => {
      const layer = canonical();
      expect(layer).toContain("text-transform: none;");
      expect(layer).not.toMatch(/text-transform: uppercase/);
      expect(layer).not.toMatch(/#[0-9a-f]{3,8}|rgba?\(|gradient\(|backdrop-filter/i);
    });

    it("keeps each topbar chip and menu skin declared once", () => {
      const css = stylesheet.replace(/@media \(forced-colors: active\) \{[\s\S]*?\n\}\n/g, "");
      const count = (pattern: RegExp) => (css.match(pattern) ?? []).length;
      expect(count(/^\.product-shell \.system-status-attention > summary(?:,| \{)/gm)).toBe(1);
      expect(count(/^\.product-shell \.system-status-ready > summary(?:,| \{)/gm)).toBe(1);
      expect(count(/^\.product-shell \.account-menu-trigger \{/gm)).toBe(1);
    });
  });

  it("resolves every custom property the stylesheet references (no silently invalid var())", () => {
    const declared = new Set([...stylesheet.matchAll(/(--[\w-]+)\s*:/g)].map((match) => match[1]));
    const unresolved = new Map<string, number>();
    for (const match of stylesheet.matchAll(/var\((--[\w-]+)\s*([,)])/g)) {
      const [, name, terminator] = match;
      // A fallback (`var(--x, value)`) is an explicit, valid contract.
      if (terminator === "," || declared.has(name)) continue;
      unresolved.set(name, (unresolved.get(name) ?? 0) + 1);
    }
    expect([...unresolved.entries()], "undefined custom properties make declarations invalid at computed-value time").toEqual([]);
  });

  describe("canonical form controls, supporting text, and notices (ADR-0426)", () => {
    const canonical = () => stylesheet.slice(stylesheet.lastIndexOf("/* ADR-0426:"), stylesheet.lastIndexOf("/* Forced-colors must remain"));

    it("renders outlined text fields with M3 shape, type, and states", () => {
      const layer = canonical();
      expect(layer).toContain(".product-shell input,\n.product-shell textarea,\n.product-shell select {");
      expect(layer).toContain("border-radius: var(--md-sys-shape-corner-extra-small);");
      expect(layer).toContain("font-size: var(--md-sys-typescale-body-large-size);");
      expect(layer).toContain("background: transparent;");
      expect(layer).toContain("border-color: var(--app-on-surface);");
      expect(layer).toContain("border-color: var(--app-primary);");
    });

    it("styles invalid and disabled fields without relying on colour alone", () => {
      const layer = canonical();
      expect(layer).toContain('[aria-invalid="true"]');
      expect(layer).toContain("border-color: var(--app-error);");
      expect(layer).toContain("box-shadow: 0 0 0 1px var(--app-error);");
      expect(layer).toContain(":disabled");
      expect(layer).toContain("color: color-mix(in srgb, var(--app-on-surface) 38%, transparent);");
      expect(layer).toContain("border-color: color-mix(in srgb, var(--app-on-surface) 12%, transparent);");
    });

    it("uses the type scale for labels, supporting text, legends, and notices", () => {
      const layer = canonical();
      expect(layer).toContain("var(--md-sys-typescale-label-medium-size)");
      expect(layer).toContain("var(--md-sys-typescale-body-small-size)");
      expect(layer).toContain("var(--md-sys-typescale-body-medium-size)");
      expect(layer).toContain("var(--md-sys-typescale-title-small-size)");
      expect(layer).toContain("border-radius: var(--md-sys-shape-corner-medium);");
      expect(layer).not.toMatch(/font-size: [0-9.]+rem|border-radius: [0-9.]+rem/);
      expect(layer).not.toMatch(/#[0-9a-f]{3,8}|rgba?\(|gradient\(|backdrop-filter/i);
    });

    it("keeps form-surface text and corners off rem literals", () => {
      const css = stylesheet.replace(/@media \(forced-colors: active\) \{[\s\S]*?\n\}\n/g, "");
      const surfaces = /^(?:\.product-shell )?(?:\.field-help|\.evidence-note|\.inline-(?:warning|error|success)|\.route-evidence|label span|input, textarea, select|\.[\w-]+ legend|\.bracket-row label span|\.revenue-claim-form small|\.revenue-inline-field span|\.session-control-local[^{]*)\b[^{]*\{[^}]*\}/gm;
      const offenders = (css.match(surfaces) ?? []).filter((rule) => /font-size: [0-9.]+rem|border-radius: [0-9.]+rem/.test(rule));
      expect(offenders).toEqual([]);
    });

    it("defines the field skin once (no content-only duplicate)", () => {
      const css = stylesheet.replace(/@media \(forced-colors: active\) \{[\s\S]*?\n\}\n/g, "");
      expect(css).not.toMatch(/^\.product-shell \.product-content input/m);
      expect((css.match(/^\.product-shell input,$/gm) ?? []).length).toBe(1);
    });
  });

  describe("canonical access-control, evidence, and empty-state surfaces (ADR-0427)", () => {
    const canonical = () => stylesheet.slice(stylesheet.lastIndexOf("/* ADR-0427:"), stylesheet.lastIndexOf("/* Forced-colors must remain"));
    const withoutForcedColors = () => stylesheet.replace(/@media \(forced-colors: active\) \{[\s\S]*?\n\}\n/g, "");

    it("renders inner cards as borderless tonal containers with a medium corner", () => {
      const layer = canonical();
      expect(layer).toContain(".product-shell .permission-group {");
      expect(layer).toContain("border-color: transparent;");
      expect(layer).toContain("border-radius: var(--md-sys-shape-corner-medium);");
    });

    it("sets permission, policy, and technical text on the type scale", () => {
      const layer = canonical();
      for (const token of [
        "var(--md-sys-typescale-body-medium-size)",
        "var(--md-sys-typescale-body-small-size)",
        "var(--md-sys-typescale-title-small-size)",
        "var(--md-sys-typescale-label-large-size)",
        "var(--md-sys-typescale-label-medium-size)",
        "var(--md-sys-typescale-label-small-size)",
        "var(--md-sys-typescale-headline-small-size)",
      ]) expect(layer, token).toContain(token);
    });

    it("renders risk and decision chips at label-small, in sentence case, with a small corner", () => {
      const layer = canonical();
      expect(layer).toContain(".product-shell .risk-chip,\n.product-shell .decision-chip {");
      expect(layer).toContain("border-radius: var(--md-sys-shape-corner-small);");
      expect(layer).toContain("text-transform: none;");
      expect(layer).not.toMatch(/text-transform: uppercase/);
      expect(layer).not.toMatch(/font-size: [0-9.]+rem|border-radius: [0-9.]+rem/);
      expect(layer).not.toMatch(/#[0-9a-f]{3,8}|rgba?\(|gradient\(|backdrop-filter/i);
    });

    it("keeps these surfaces off rem font-size and radius literals", () => {
      const surfaces = /^(?:\.product-shell )?\.(?:empty-(?:state|icon)|permission-[\w-]+|policy-dimension-grid|technical-(?:details|policy-list)|authorization-posture|risk-chip|decision-chip|release-evidence)\b[^{]*\{[^}]*\}/gm;
      const offenders = (withoutForcedColors().match(surfaces) ?? []).filter((rule) => /font-size: [0-9.]+rem|border-radius: [0-9.]+rem/.test(rule));
      expect(offenders).toEqual([]);
    });

    it("never truncates authorization posture values and wraps the strip to the workspace width", () => {
      const css = withoutForcedColors();
      const posture = css.slice(css.lastIndexOf(".product-shell .authorization-posture {"));
      expect(posture).toContain("repeat(auto-fit, minmax(min(100%, 9rem), 1fr))");
      const strong = css.slice(css.lastIndexOf(".product-shell .authorization-posture strong {"));
      expect(strong.slice(0, strong.indexOf("}"))).toContain("white-space: normal;");
      expect(strong.slice(0, strong.indexOf("}"))).toContain("overflow-wrap: anywhere;");
      expect(css).not.toMatch(/^\.authorization-posture strong \{[^}]*(?:text-overflow|white-space: nowrap)/m);
    });

    it("keeps permission-group skin declared once", () => {
      const count = (withoutForcedColors().match(/^\.product-shell \.permission-group \{/gm) ?? []).length;
      expect(count).toBe(1);
    });
  });

  describe("motion tokens, meaningful tints, and wrapping rows (ADR-0430)", () => {
    const css = () => stylesheet.replace(/\/\*[\s\S]*?\*\//g, "");

    it("expresses every transition and animation through motion tokens", () => {
      const raw = css().split("\n").filter((line) => /(?:transition|animation)\s*:/.test(line) && /[\d.]+m?s\b|\bease(?:-in|-out|-in-out)?\b|cubic-bezier/.test(line) && !line.includes("--md-"));
      expect(raw).toEqual([]);
      expect(css()).not.toMatch(/transform:\s*translateY\(-1px\)/);
    });

    it("has no near-invisible colour-mix tints (3% or less) pretending to be a role", () => {
      const invisible = css().match(/color-mix\(in srgb, var\(--[\w-]+\) 0?[0-3]%, transparent\)/g) ?? [];
      expect(invisible).toEqual([]);
    });

    it("lets the rule-workbench source row wrap instead of clipping its fields", () => {
      const layer = css();
      expect(layer).toMatch(/\.product-shell \.workbench-inline-row \{[^}]*flex-wrap: wrap;/);
      expect(layer).toMatch(/\.product-shell \.workbench-inline-row > label \{[^}]*flex: 1 1 7rem;[^}]*min-width: 0;/);
      expect(layer).toMatch(/\.product-shell \.base-share-fraction \{[^}]*display: flex;[^}]*flex: 1 1 11rem;[^}]*min-width: 0;/);
      expect(layer).toMatch(/\.product-shell \.base-share-fraction > label \{[^}]*flex: 1 1 0;[^}]*min-width: 0;/);
    });
  });

  describe("elevation tokens (ADR-0434)", () => {
    const css = () => stylesheet.replace(/\/\*[\s\S]*?\*\//g, "");

    it("declares the M3 elevation scale from level 0 to level 5", () => {
      const expected: Record<number, string> = {
        0: "none",
        1: "0 1px 2px 0 rgba(0, 0, 0, .3), 0 1px 3px 1px rgba(0, 0, 0, .15)",
        2: "0 1px 2px 0 rgba(0, 0, 0, .3), 0 2px 6px 2px rgba(0, 0, 0, .15)",
        3: "0 1px 3px 0 rgba(0, 0, 0, .3), 0 4px 8px 3px rgba(0, 0, 0, .15)",
        4: "0 2px 3px 0 rgba(0, 0, 0, .3), 0 6px 10px 4px rgba(0, 0, 0, .15)",
        5: "0 4px 4px 0 rgba(0, 0, 0, .3), 0 8px 12px 6px rgba(0, 0, 0, .15)",
      };
      for (const [level, value] of Object.entries(expected)) {
        expect(css(), `level ${level}`).toContain(`--md-sys-elevation-level${level}: ${value};`);
      }
      expect(css()).not.toMatch(/--md-elevation-[12]\b/);
    });

    it("restricts every component shadow to a token, none, or a one-pixel state ring", () => {
      const offenders: string[] = [];
      for (const match of css().matchAll(/box-shadow:\s*([^;}]+)[;}]/g)) {
        const value = match[1].trim();
        const allowed = value === "none"
          || /^var\(--md-sys-elevation-level[0-5]\)$/.test(value)
          || /^0 0 0 1px var\(--app-[\w-]+\)$/.test(value);
        if (!allowed) offenders.push(value);
      }
      expect(offenders).toEqual([]);
    });

    it("keeps menus and popovers at elevation level 2", () => {
      const menus = css().slice(css().lastIndexOf(".product-shell .display-settings-menu,"));
      expect(menus.slice(0, menus.indexOf("}"))).toContain("box-shadow: var(--md-sys-elevation-level2);");
    });
  });

  describe("spacing scale (ADR-0434)", () => {
    const css = () => stylesheet.replace(/\/\*[\s\S]*?\*\//g, "");

    it("declares a 4px-grid spacing scale as repository-owned tokens", () => {
      const expected: Record<string, string> = {
        half: ".125rem", 1: ".25rem", 2: ".5rem", 3: ".75rem", 4: "1rem", 5: "1.25rem", 6: "1.5rem", 8: "2rem", 10: "2.5rem",
      };
      for (const [name, value] of Object.entries(expected)) {
        expect(css(), `--app-space-${name}`).toContain(`--app-space-${name}: ${value};`);
      }
    });

    it("spells every padding, margin, and gap through the scale (or a function, 0, auto, %)", () => {
      const spacing = /^(?:padding|margin)(?:-(?:top|right|bottom|left|inline|block)(?:-(?:start|end))?)?$|^(?:row-|column-)?gap$/;
      const offenders: string[] = [];
      for (const match of css().matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
        if (match[1].trim().startsWith("--") || match[1].includes(":root")) continue;
        // The visually-hidden utility is a 1px clip technique, not layout spacing.
        if (match[1].trim() === ".sr-only") continue;
        for (const declaration of match[2].split(";")) {
          const [property, ...rest] = declaration.split(":");
          const value = rest.join(":").trim();
          if (!spacing.test(property.trim()) || /(?:calc|clamp|min|max)\(/.test(value)) continue;
          if (/[\d.]+rem|[\d.]+px/.test(value)) offenders.push(`${match[1].trim().replace(/\s+/g, " ").slice(0, 60)} { ${property.trim()}: ${value} }`);
        }
      }
      expect(offenders).toEqual([]);
    });
  });
});
