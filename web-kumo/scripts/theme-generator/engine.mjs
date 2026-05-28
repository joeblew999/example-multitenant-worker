/**
 * Theme-CSS emission engine — pure logic, no I/O.
 *
 * generate.mjs invokes `emitPaletteCSS(scenario)` and `emitMappingsCSS(scenario)`
 * for each discovered scenarios/<name>/scenario.mjs. The scenario module
 * shape is documented in src/seed-scenarios/types.ts.
 *
 * Output is unlayered + scoped to `[data-theme="<name>"]` so multiple
 * themes coexist on the same page (see KUMO.md §3 + §11).
 */

import { THEME_CONFIG as KUMO_CONFIG } from "@cloudflare/kumo/scripts/theme-generator/config";

/** @param {Record<string, string>} obj @param {string} indent */
function emitVars(obj, indent = "  ") {
  return Object.entries(obj).map(([k, v]) => `${indent}--${k}: ${v};`);
}

/**
 * Palette CSS: `[data-theme="<name>"] { --accent: ... }` + optional
 * `@media (prefers-color-scheme: light)` override scoped the same way.
 */
export function emitPaletteCSS({ SCENARIO_NAME, THEME }) {
  const header = `/**
 * AUTO-GENERATED — do not edit directly.
 *
 * Source:    scenarios/${SCENARIO_NAME}/scenario.mjs (THEME export)
 * Generator: scripts/theme-generator/generate.mjs
 * Regen:     mise run kumo:theme-gen
 *
 * Editorial palette as CSS custom properties scoped to
 * [data-theme="${SCENARIO_NAME}"] so multiple themes can coexist
 * on one page. Other files reference these vars (\`var(--accent)\`).
 */
`;
  const { palette, fonts, scale, colorScheme, colorSchemeLight } = THEME;
  const lines = [header, `[data-theme="${SCENARIO_NAME}"] {`];

  lines.push(`  /* Colors (base mode) */`);
  lines.push(...emitVars(palette.base));
  lines.push("");
  lines.push(`  /* Typography */`);
  lines.push(...emitVars(fonts));
  lines.push("");
  lines.push(`  /* Scale — type, spacing, radii, motion */`);
  lines.push(...emitVars(scale));
  if (colorScheme) {
    lines.push("");
    lines.push(`  color-scheme: ${colorScheme};`);
  }
  lines.push(`}`);
  lines.push("");

  if (palette.light && Object.keys(palette.light).length > 0) {
    lines.push(`@media (prefers-color-scheme: light) {`);
    lines.push(`  [data-theme="${SCENARIO_NAME}"] {`);
    lines.push(...emitVars(palette.light, "    "));
    if (colorSchemeLight) {
      lines.push(`    color-scheme: ${colorSchemeLight};`);
    }
    lines.push(`  }`);
    lines.push(`}`);
    lines.push("");
  }
  return lines.join("\n");
}

/**
 * Kumo --color-kumo-* mappings: `[data-theme="<name>"] { --color-kumo-brand: var(--accent); ... }`.
 * Unlayered to beat Kumo's unlayered `:root, :host` defaults (KUMO.md §3).
 */
export function emitMappingsCSS({ SCENARIO_NAME, THEME }) {
  const header = `/**
 * AUTO-GENERATED — do not edit directly.
 *
 * Source:    scenarios/${SCENARIO_NAME}/scenario.mjs (THEME.kumoOverrides)
 * Generator: scripts/theme-generator/generate.mjs
 *
 * UNLAYERED — see KUMO.md §3. Maps Kumo's --color-kumo-* tokens to the
 * ${SCENARIO_NAME} palette vars from theme-${SCENARIO_NAME}-palette.css.
 */
`;
  const lines = [header, `[data-theme="${SCENARIO_NAME}"] {`];
  const { text, color } = THEME.kumoOverrides;

  if (Object.keys(text).length) {
    lines.push(`  /* Text colors */`);
    for (const [k, v] of Object.entries(text)) {
      lines.push(`  --text-color-${k}: ${v};`);
    }
  }
  if (Object.keys(color).length) {
    if (Object.keys(text).length) lines.push("");
    lines.push(`  /* Surfaces, borders, brand, status */`);
    for (const [k, v] of Object.entries(color)) {
      lines.push(`  --color-${k}: ${v};`);
    }
  }
  lines.push(`}`);
  lines.push("");
  return lines.join("\n");
}

/** Warn (don't throw) on token names not in Kumo's THEME_CONFIG. */
export function validate({ SCENARIO_NAME, THEME }) {
  const warnings = [];
  const kumoText = new Set(Object.keys(KUMO_CONFIG.text));
  const kumoColor = new Set(Object.keys(KUMO_CONFIG.color));
  for (const name of Object.keys(THEME.kumoOverrides.text)) {
    if (!kumoText.has(name)) warnings.push(`[${SCENARIO_NAME}] unknown text token: ${name}`);
  }
  for (const name of Object.keys(THEME.kumoOverrides.color)) {
    if (!kumoColor.has(name)) warnings.push(`[${SCENARIO_NAME}] unknown color token: ${name}`);
  }
  return warnings;
}
