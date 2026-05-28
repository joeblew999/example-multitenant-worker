/**
 * Theme-CSS emission engine — pure logic, no I/O.
 *
 * generate.mjs imports this and invokes it once per theme config.
 * Each theme config (config.editorial.mjs, config.remysport.mjs, etc.)
 * exports THEME_NAME + PALETTE + FONTS + SCALE + EDITORIAL_OVERRIDES,
 * and this engine turns that into:
 *
 *   1. theme-<name>-palette.css — [data-theme="<name>"] vars + a
 *      `@media (prefers-color-scheme: light)` override scoped the
 *      same way. Scoped (not :root) so multiple themes coexist on
 *      one page; only the active data-theme's vars resolve.
 *
 *   2. theme-<name>.css — [data-theme="<name>"] block mapping Kumo's
 *      --color-kumo-* tokens to the palette via var() refs. Unlayered
 *      so it beats Kumo's unlayered :root/:host defaults (KUMO.md §3).
 *
 * @typedef {import("@cloudflare/kumo/scripts/theme-generator/types").ThemeConfig} ThemeConfig
 */

import { THEME_CONFIG as KUMO_CONFIG } from "@cloudflare/kumo/scripts/theme-generator/config";

/**
 * @param {Record<string, string>} obj
 * @param {string} indent
 */
function emitVars(obj, indent = "  ") {
  return Object.entries(obj).map(([k, v]) => `${indent}--${k}: ${v};`);
}

/**
 * Build the palette CSS string for a theme config.
 *
 * Scoping note: `[data-theme="X"]` (specificity 0,1,0) rather than
 * `:root` (specificity 0,0,1) because we want multiple themes' palettes
 * to coexist in the same stylesheet. Whichever theme is active on
 * <html> wins for descendant `var(--accent)` etc. lookups via normal
 * CSS variable inheritance.
 */
export function emitPaletteCSS({ THEME_NAME, PALETTE, FONTS, SCALE }) {
  const header = `/**
 * AUTO-GENERATED — do not edit directly.
 *
 * Source:    scripts/theme-generator/config.${THEME_NAME}.mjs
 *            (PALETTE + FONTS + SCALE exports)
 * Generator: scripts/theme-generator/generate.mjs
 * Regen:     mise run kumo:theme-gen
 *
 * Declares the ${THEME_NAME} palette as CSS custom properties scoped to
 * \`[data-theme="${THEME_NAME}"]\` so multiple themes can coexist on the
 * same page. Every other file in the stack references these vars rather
 * than inlining hex values; the palette is the single source of truth
 * in config.${THEME_NAME}.mjs.
 */
`;
  const lines = [header, `[data-theme="${THEME_NAME}"] {`];

  lines.push(`  /* Colors (base mode) */`);
  lines.push(...emitVars(PALETTE.base));
  lines.push("");
  lines.push(`  /* Typography */`);
  lines.push(...emitVars(FONTS));
  lines.push("");
  lines.push(`  /* Scale — type, spacing, radii, motion */`);
  lines.push(...emitVars(SCALE));
  if (PALETTE.colorScheme) {
    lines.push("");
    lines.push(`  color-scheme: ${PALETTE.colorScheme};`);
  }
  lines.push(`}`);
  lines.push("");

  if (PALETTE.light && Object.keys(PALETTE.light).length > 0) {
    lines.push(`@media (prefers-color-scheme: light) {`);
    lines.push(`  [data-theme="${THEME_NAME}"] {`);
    lines.push(...emitVars(PALETTE.light, "    "));
    if (PALETTE.colorSchemeLight) {
      lines.push(`    color-scheme: ${PALETTE.colorSchemeLight};`);
    }
    lines.push(`  }`);
    lines.push(`}`);
    lines.push("");
  }

  return lines.join("\n");
}

/**
 * @param {"text" | "color"} group
 * @param {Record<string, { theme: Record<string, { light: string; dark: string }> }>} tokens
 * @param {string} themeName
 * @param {string} prefix
 */
function emitGroup(group, tokens, themeName, prefix) {
  const lines = [];
  for (const [name, def] of Object.entries(tokens)) {
    const value = def.theme[themeName]?.light ?? "";
    if (!value) continue;
    lines.push(`  --${prefix}-${name}: ${value};`);
  }
  return lines;
}

/**
 * Build the Kumo-mappings CSS string for a theme config.
 */
export function emitMappingsCSS({ THEME_NAME, KUMO_OVERRIDES }) {
  const header = `/**
 * AUTO-GENERATED — do not edit directly.
 *
 * Source:    scripts/theme-generator/config.${THEME_NAME}.mjs
 *            (KUMO_OVERRIDES export)
 * Generator: scripts/theme-generator/generate.mjs
 * Regen:     mise run kumo:theme-gen
 *
 * UNLAYERED on purpose — see KUMO.md §3 for why \`@layer base\` would
 * let Kumo's unlayered \`:root, :host\` defaults silently win.
 * Maps Kumo's --color-kumo-* tokens to the ${THEME_NAME} palette vars
 * declared in theme-${THEME_NAME}-palette.css.
 */
`;
  const lines = [header, `[data-theme="${THEME_NAME}"] {`];

  const text = emitGroup("text", KUMO_OVERRIDES.text, THEME_NAME, "text-color");
  const color = emitGroup("color", KUMO_OVERRIDES.color, THEME_NAME, "color");

  if (text.length) {
    lines.push(`  /* Text colors */`);
    lines.push(...text);
  }
  if (color.length) {
    if (text.length) lines.push("");
    lines.push(`  /* Surfaces, borders, brand, status */`);
    lines.push(...color);
  }

  lines.push(`}`);
  lines.push("");

  return lines.join("\n");
}

/**
 * Cross-check token names against Kumo's THEME_CONFIG. Warns (doesn't
 * throw) so typos are caught without blocking regen.
 */
export function validate({ THEME_NAME, KUMO_OVERRIDES }) {
  const warnings = [];
  for (const group of ["text", "color"]) {
    const kumoTokens = new Set(Object.keys(KUMO_CONFIG[group]));
    for (const name of Object.keys(KUMO_OVERRIDES[group])) {
      if (!kumoTokens.has(name)) {
        warnings.push(`[${THEME_NAME}] unknown ${group} token: ${name}`);
      }
    }
  }
  return warnings;
}
