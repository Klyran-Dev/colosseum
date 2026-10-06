import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";

const repositoryRoot = resolve(fileURLToPath(new URL("..", import.meta.url)));
const stylesheetPath = process.env.MATERIAL_STYLES_PATH ?? resolve(repositoryRoot, "apps/web/src/styles.css");
const stylesheet = await readFile(stylesheetPath, "utf8");

const requiredPairs = [
  ["background", "on-background"],
  ["surface", "on-surface"],
  ["surface-variant", "on-surface-variant"],
  ["surface-container", "on-surface"],
  ["surface-container-lowest", "on-surface"],
  ["surface-container-high", "on-surface"],
  ["surface-container-highest", "on-surface"],
  ["primary", "on-primary"],
  ["primary-container", "on-primary-container"],
  ["secondary", "on-secondary"],
  ["secondary-container", "on-secondary-container"],
  ["tertiary", "on-tertiary"],
  ["tertiary-container", "on-tertiary-container"],
  ["error", "on-error"],
  ["error-container", "on-error-container"],
  ["warning", "on-warning"],
  ["warning-container", "on-warning-container"],
  ["surface-container-low", "on-surface"],
  ["surface-container-low", "on-surface-variant"],
  ["surface-container", "on-surface-variant"],
  ["surface-container-high", "on-surface-variant"],
  ["surface-container-highest", "on-surface-variant"],
  ["surface-dim", "on-surface"],
  ["surface-bright", "on-surface"],
  ["inverse-surface", "inverse-on-surface"],
  ["primary-fixed", "on-primary-fixed"],
  ["primary-fixed-dim", "on-primary-fixed"],
  ["primary-fixed", "on-primary-fixed-variant"],
  ["secondary-fixed", "on-secondary-fixed"],
  ["secondary-fixed-dim", "on-secondary-fixed"],
  ["secondary-fixed", "on-secondary-fixed-variant"],
  ["tertiary-fixed", "on-tertiary-fixed"],
  ["tertiary-fixed-dim", "on-tertiary-fixed"],
  ["tertiary-fixed", "on-tertiary-fixed-variant"],
];
const minimumContrast = 4.5;

const requiredThemeMarker = "Material 3 Expressive theme family: Material Baseline / seed #6750A4 / TonalSpot / role matrix v3";
const expectedRoleValues = {
  dark: {
    "primary": "#CFBDFE",
    "on-primary": "#36275D",
    "primary-container": "#4D3D75",
    "on-primary-container": "#E9DDFF",
    "secondary": "#CBC2DB",
    "on-secondary": "#332D41",
    "secondary-container": "#4A4458",
    "on-secondary-container": "#E8DEF8",
    "tertiary": "#EFB8C8",
    "on-tertiary": "#4A2532",
    "tertiary-container": "#633B48",
    "on-tertiary-container": "#FFD9E3",
    "error": "#FFB4AB",
    "on-error": "#690005",
    "error-container": "#93000A",
    "on-error-container": "#FFDAD6",
    "background": "#141218",
    "on-background": "#E6E0E9",
    "surface": "#141218",
    "on-surface": "#E6E0E9",
    "surface-variant": "#49454E",
    "on-surface-variant": "#CAC4CF",
    "surface-dim": "#141218",
    "surface-bright": "#3B383E",
    "surface-container-lowest": "#0F0D13",
    "surface-container-low": "#1D1B20",
    "surface-container": "#211F24",
    "surface-container-high": "#2B292F",
    "surface-container-highest": "#36343A",
    "outline": "#948F99",
    "outline-variant": "#49454E",
    "inverse-surface": "#E6E0E9",
    "inverse-on-surface": "#322F35",
    "inverse-primary": "#65558F",
    "surface-tint": "#CFBDFE",
    "scrim": "#000000",
    "shadow": "#000000",
    "primary-fixed": "#E9DDFF",
    "primary-fixed-dim": "#CFBDFE",
    "on-primary-fixed": "#201047",
    "on-primary-fixed-variant": "#4D3D75",
    "secondary-fixed": "#E8DEF8",
    "secondary-fixed-dim": "#CBC2DB",
    "on-secondary-fixed": "#1E192B",
    "on-secondary-fixed-variant": "#4A4458",
    "tertiary-fixed": "#FFD9E3",
    "tertiary-fixed-dim": "#EFB8C8",
    "on-tertiary-fixed": "#31101D",
    "on-tertiary-fixed-variant": "#633B48",
    "warning": "#FFB870",
    "on-warning": "#4A2800",
    "warning-container": "#693C00",
    "on-warning-container": "#FFDCBE",
  },
  light: {
    "primary": "#65558F",
    "on-primary": "#FFFFFF",
    "primary-container": "#E9DDFF",
    "on-primary-container": "#4D3D75",
    "secondary": "#625B71",
    "on-secondary": "#FFFFFF",
    "secondary-container": "#E8DEF8",
    "on-secondary-container": "#4A4458",
    "tertiary": "#7E5260",
    "on-tertiary": "#FFFFFF",
    "tertiary-container": "#FFD9E3",
    "on-tertiary-container": "#633B48",
    "error": "#BA1A1A",
    "on-error": "#FFFFFF",
    "error-container": "#FFDAD6",
    "on-error-container": "#93000A",
    "background": "#FDF7FF",
    "on-background": "#1D1B20",
    "surface": "#FDF7FF",
    "on-surface": "#1D1B20",
    "surface-variant": "#E7E0EB",
    "on-surface-variant": "#49454E",
    "surface-dim": "#DED8E0",
    "surface-bright": "#FDF7FF",
    "surface-container-lowest": "#FFFFFF",
    "surface-container-low": "#F8F2FA",
    "surface-container": "#F2ECF4",
    "surface-container-high": "#ECE6EE",
    "surface-container-highest": "#E6E0E9",
    "outline": "#7A757F",
    "outline-variant": "#CAC4CF",
    "inverse-surface": "#322F35",
    "inverse-on-surface": "#F5EFF7",
    "inverse-primary": "#CFBDFE",
    "surface-tint": "#65558F",
    "scrim": "#000000",
    "shadow": "#000000",
    "primary-fixed": "#E9DDFF",
    "primary-fixed-dim": "#CFBDFE",
    "on-primary-fixed": "#201047",
    "on-primary-fixed-variant": "#4D3D75",
    "secondary-fixed": "#E8DEF8",
    "secondary-fixed-dim": "#CBC2DB",
    "on-secondary-fixed": "#1E192B",
    "on-secondary-fixed-variant": "#4A4458",
    "tertiary-fixed": "#FFD9E3",
    "tertiary-fixed-dim": "#EFB8C8",
    "on-tertiary-fixed": "#31101D",
    "on-tertiary-fixed-variant": "#633B48",
    "warning": "#8B5000",
    "on-warning": "#FFFFFF",
    "warning-container": "#FFDCBE",
    "on-warning-container": "#2C1600",
  },
};

function fail(message) {
  console.error(`Material palette contract failed: ${message}`);
  process.exit(1);
}

function parseHex(value, role, scheme) {
  if (!/^#[0-9a-f]{6}$/i.test(value)) fail(`${scheme} ${role} must be a six-digit hex color`);
  return [1, 3, 5].map((offset) => Number.parseInt(value.slice(offset, offset + 2), 16) / 255);
}

function linearize(value) {
  return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
}

function luminance(rgb) {
  return 0.2126 * linearize(rgb[0]) + 0.7152 * linearize(rgb[1]) + 0.0722 * linearize(rgb[2]);
}

function contrastRatio(first, second) {
  const firstLuminance = luminance(first);
  const secondLuminance = luminance(second);
  return (Math.max(firstLuminance, secondLuminance) + 0.05) / (Math.min(firstLuminance, secondLuminance) + 0.05);
}

const schemes = [
  ["dark", /^:root\s*\{([\s\S]*?)\n\}/m],
  ["light", /^:root\[data-theme="light"\]\s*\{([\s\S]*?)\n\}/m],
];

if (!stylesheet.includes(requiredThemeMarker)) {
  fail(`missing exact theme marker: ${requiredThemeMarker}`);
}

for (const [scheme, blockPattern] of schemes) {
  const block = stylesheet.match(blockPattern)?.[1];
  if (!block) fail(`missing ${scheme} token block`);
  const tokens = new Map();
  for (const match of block.matchAll(/--md-sys-color-([\w-]+):\s*(#[0-9a-f]{6})\s*;/gi)) {
    tokens.set(match[1], match[2]);
  }
  for (const [backgroundRole, foregroundRole] of requiredPairs) {
    const background = tokens.get(backgroundRole);
    const foreground = tokens.get(foregroundRole);
    if (!background || !foreground) fail(`missing ${scheme} role pair ${backgroundRole}/${foregroundRole}`);
    const ratio = contrastRatio(parseHex(background, backgroundRole, scheme), parseHex(foreground, foregroundRole, scheme));
    if (ratio < minimumContrast) {
      fail(`${scheme} ${backgroundRole}/${foregroundRole} contrast ${ratio.toFixed(2)} is below ${minimumContrast.toFixed(1)}`);
    }
  }
  for (const [role, expected] of Object.entries(expectedRoleValues[scheme])) {
    const actual = tokens.get(role)?.toUpperCase();
    if (actual !== expected) fail(`${scheme} ${role} must be ${expected}, found ${actual ?? "missing"}`);
  }
}

console.log(`Material palette contrast contract passed (${requiredPairs.length} role pairs, ${minimumContrast.toFixed(1)}:1 minimum)`);
