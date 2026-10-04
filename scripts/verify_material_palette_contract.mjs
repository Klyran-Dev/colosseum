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
];
const minimumContrast = 4.5;

const requiredThemeMarker = "Material 3 Expressive theme family: Solana Aurora / seed #8DF0CE / role matrix v1";
const expectedRoleValues = {
  dark: {
    primary: "#8DF0CE",
    "on-primary": "#00382D",
    "primary-container": "#005143",
    "on-primary-container": "#ACF2DD",
    secondary: "#B3CCC3",
    "on-secondary": "#1F3530",
    "secondary-container": "#354B44",
    "on-secondary-container": "#CEE9E0",
    tertiary: "#A9C8FF",
    "on-tertiary": "#0A305F",
    "tertiary-container": "#284777",
    "on-tertiary-container": "#D5E3FF",
    error: "#FFB4AB",
    "on-error": "#690005",
    "error-container": "#93000A",
    "on-error-container": "#FFDAD6",
    warning: "#F5C467",
    "on-warning": "#3F2E00",
    "warning-container": "#5D4300",
    "on-warning-container": "#FFDEA0",
    background: "#071321",
    "on-background": "#E1E8F5",
    surface: "#071321",
    "on-surface": "#E1E8F5",
    "surface-variant": "#424852",
    "on-surface-variant": "#C4CAD5",
    "surface-container": "#0D1B2F",
    "surface-container-lowest": "#020B16",
    "surface-container-low": "#0A1728",
    "surface-container-high": "#142943",
    "surface-container-highest": "#1D3654",
    "surface-dim": "#071321",
    "surface-bright": "#1D3654",
    outline: "#8D94A1",
    "outline-variant": "#424852",
    "inverse-surface": "#E1E8F5",
    "inverse-on-surface": "#2D3037",
    "inverse-primary": "#006B5A",
  },
  light: {
    primary: "#006B5A",
    "on-primary": "#FFFFFF",
    "primary-container": "#8DF0CE",
    "on-primary-container": "#002019",
    secondary: "#4E635C",
    "on-secondary": "#FFFFFF",
    "secondary-container": "#D0E8DF",
    "on-secondary-container": "#0A1F19",
    tertiary: "#4C5F8A",
    "on-tertiary": "#FFFFFF",
    "tertiary-container": "#D5E3FF",
    "on-tertiary-container": "#091B36",
    error: "#BA1A1A",
    "on-error": "#FFFFFF",
    "error-container": "#FFDAD6",
    "on-error-container": "#410002",
    warning: "#775A00",
    "on-warning": "#FFFFFF",
    "warning-container": "#FFDEA0",
    "on-warning-container": "#261A00",
    background: "#F8F9FF",
    "on-background": "#181B22",
    surface: "#F8F9FF",
    "on-surface": "#181B22",
    "surface-variant": "#DFE4EC",
    "on-surface-variant": "#43474F",
    "surface-container": "#ECEFF7",
    "surface-container-lowest": "#FFFFFF",
    "surface-container-low": "#F2F5FC",
    "surface-container-high": "#E6EAF3",
    "surface-container-highest": "#E0E5EF",
    "surface-dim": "#D5D8E0",
    "surface-bright": "#F8F9FF",
    outline: "#737780",
    "outline-variant": "#C3C7D0",
    "inverse-surface": "#2D3037",
    "inverse-on-surface": "#F0F2F9",
    "inverse-primary": "#7CD9BE",
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
