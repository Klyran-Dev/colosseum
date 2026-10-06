// Deterministic Material 3 palette generator (ADR-0422).
//
// Inputs are pinned: seed #6750A4 (the Material baseline), the library's
// default TonalSpot variant at standard contrast, and the exact
// @material/material-color-utilities version in package.json. The stylesheet
// token blocks are verified against this output by
// src/materialPaletteGeneration.test.ts; nothing here ships to browsers.
import {
  argbFromHex,
  Blend,
  hexFromArgb,
  Hct,
  MaterialDynamicColors as Roles,
  SchemeTonalSpot,
  TonalPalette,
} from "@material/material-color-utilities";

export const SEED = "#6750A4";
// Material defines no warning role. Amber is harmonized toward the seed so it
// sits in the same palette family while staying distinguishable from error.
export const WARNING_SOURCE = "#FFB300";
export const CONTRAST_LEVEL = 0;

const kebab = (name) => name.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);

const ROLE_NAMES = [
  "primary", "onPrimary", "primaryContainer", "onPrimaryContainer",
  "secondary", "onSecondary", "secondaryContainer", "onSecondaryContainer",
  "tertiary", "onTertiary", "tertiaryContainer", "onTertiaryContainer",
  "error", "onError", "errorContainer", "onErrorContainer",
  "background", "onBackground", "surface", "onSurface",
  "surfaceVariant", "onSurfaceVariant",
  "surfaceDim", "surfaceBright",
  "surfaceContainerLowest", "surfaceContainerLow", "surfaceContainer",
  "surfaceContainerHigh", "surfaceContainerHighest",
  "outline", "outlineVariant",
  "inverseSurface", "inverseOnSurface", "inversePrimary",
  "surfaceTint", "scrim", "shadow",
  "primaryFixed", "primaryFixedDim", "onPrimaryFixed", "onPrimaryFixedVariant",
  "secondaryFixed", "secondaryFixedDim", "onSecondaryFixed", "onSecondaryFixedVariant",
  "tertiaryFixed", "tertiaryFixedDim", "onTertiaryFixed", "onTertiaryFixedVariant",
];

export function generatePalette() {
  const source = argbFromHex(SEED);
  const warningSeed = Blend.harmonize(argbFromHex(WARNING_SOURCE), source);
  const warning = TonalPalette.fromInt(warningSeed);
  const warningTones = {
    light: { warning: 40, "on-warning": 100, "warning-container": 90, "on-warning-container": 10 },
    dark: { warning: 80, "on-warning": 20, "warning-container": 30, "on-warning-container": 90 },
  };
  const result = {};
  for (const scheme of ["light", "dark"]) {
    const dynamic = new SchemeTonalSpot(Hct.fromInt(source), scheme === "dark", CONTRAST_LEVEL);
    const roles = {};
    for (const name of ROLE_NAMES) roles[kebab(name)] = hexFromArgb(Roles[name].getArgb(dynamic)).toUpperCase();
    for (const [role, tone] of Object.entries(warningTones[scheme])) roles[role] = hexFromArgb(warning.tone(tone)).toUpperCase();
    result[scheme] = roles;
  }
  return result;
}
