#!/usr/bin/env node
/**
 * Editorial theme — CSS generator.
 *
 * Reads ./config.editorial.mjs and emits TWO CSS files:
 *
 *   1. src/styles/theme-editorial-palette.css
 *      → :root vars (PALETTE.base + FONTS + SCALE) plus a
 *        `@media (prefers-color-scheme: light)` override (PALETTE.light).
 *        These are the vars every other CSS file references.
 *
 *   2. src/styles/theme-editorial.css
 *      → `[data-theme="editorial"]` rules mapping Kumo's `--color-kumo-*`
 *        tokens to the palette vars via `var(--...)`.
 *
 * ============================================================
 * Why this exists instead of calling Kumo's generator directly
 * ============================================================
 *
 * Kumo's `scripts/theme-generator/generate-css.ts` has a clean
 * `generateThemeOverrideCSS()` function but it's a Kumo-internal
 * build helper, not a consumer API:
 *
 *   1. Kumo's `dist/scripts/theme-generator/generate-css.d.ts` ships
 *      only the type defs — there is no `generate-css.js` runtime.
 *   2. The package.json `exports` map only exposes
 *      `./scripts/theme-generator/{config,types}`. Deep imports past
 *      those fail under strict ESM resolution.
 *   3. Kumo invokes the generator via `tsx scripts/theme-generator/index.ts`
 *      from inside their own monorepo — that path is build-internal.
 *
 * So adding a custom theme means writing a generator like this one.
 *
 * ============================================================
 * Why the Kumo-mapping output is UNLAYERED (no @layer wrapper)
 * ============================================================
 *
 * Kumo's bundled CSS emits an unlayered `:root, :host` rule that sets
 * every `--color-kumo-*` token to a default value (e.g. brand → blue
 * via `light-dark(oklch(...), oklch(...))`). Per the CSS cascade-
 * layers spec, unlayered rules ALWAYS beat any layered rule
 * regardless of selector specificity.
 *
 * Kumo's own `generateThemeOverrideCSS` wraps overrides in
 * `@layer base { [data-theme="X"] { ... } }`. That means ANY theme
 * trying to override a token Kumo's `:root, :host` default sets will
 * silently lose. (Verified by switching to `<html data-theme="fedramp">`
 * in a stock Kumo dev server — `--color-kumo-brand` still paints
 * Kumo blue, not fedramp's intended color.)
 *
 * We emit `[data-theme="editorial"] { ... }` unlayered so it ties
 * Kumo's `:root, :host` on layer (both none) and wins on specificity
 * (0,1,0 > :root's 0,0,1) + source order.
 */

import { writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

import { THEME_CONFIG as KUMO_CONFIG } from "@cloudflare/kumo/scripts/theme-generator/config";
import {
  EDITORIAL_OVERRIDES,
  FONTS,
  PALETTE,
  SCALE,
  THEME_NAME,
} from "./config.editorial.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const STYLES_DIR = join(HERE, "..", "..", "src", "styles");
const PALETTE_OUT = join(STYLES_DIR, "theme-editorial-palette.css");
const MAPPINGS_OUT = join(STYLES_DIR, "theme-editorial.css");

const PALETTE_HEADER = `/**
 * AUTO-GENERATED — do not edit directly.
 *
 * Source:    scripts/theme-generator/config.editorial.mjs
 *            (PALETTE + FONTS + SCALE exports)
 * Generator: scripts/theme-generator/generate.mjs
 * Regen:     mise run kumo:theme-gen   (or pnpm theme:gen)
 *
 * Declares the editorial palette as CSS custom properties. Every other
 * file in the stack (theme-editorial.css, theme-editorial-extras.css,
 * layout-fixes.css, editorial-chrome.css) references these vars rather
 * than inlining hex values, so the palette has a single source of
 * truth in config.editorial.mjs.
 */
`;

const MAPPINGS_HEADER = `/**
 * AUTO-GENERATED — do not edit directly.
 *
 * Source:    scripts/theme-generator/config.editorial.mjs
 *            (EDITORIAL_OVERRIDES export)
 * Generator: scripts/theme-generator/generate.mjs
 * Regen:     mise run kumo:theme-gen   (or pnpm theme:gen)
 *
 * UNLAYERED on purpose — see generator header for why \`@layer base\`
 * would let Kumo's unlayered \`:root, :host\` defaults silently win.
 * Maps Kumo's --color-kumo-* tokens to the editorial palette vars
 * declared in theme-editorial-palette.css.
 */
`;

/**
 * Cross-check our Kumo-mapping token names against Kumo's THEME_CONFIG.
 * Catches typos and tokens that have been renamed/removed upstream.
 */
function validate() {
  const warnings = [];
  for (const group of /** @type {const} */ (["text", "color"])) {
    const kumoTokens = new Set(Object.keys(KUMO_CONFIG[group]));
    for (const name of Object.keys(EDITORIAL_OVERRIDES[group])) {
      if (!kumoTokens.has(name)) {
        warnings.push(`unknown ${group} token: ${name}`);
      }
    }
  }
  if (warnings.length) {
    console.warn("[theme:gen] WARN — token names not in kumo base:");
    for (const w of warnings) console.warn("  " + w);
  }
}

// --- File 1: theme-editorial-palette.css -----------------------------

/**
 * @param {Record<string, string>} obj
 * @param {string} indent
 */
function emitVars(obj, indent = "  ") {
  return Object.entries(obj).map(([k, v]) => `${indent}--${k}: ${v};`);
}

function emitPaletteCSS() {
  const lines = [PALETTE_HEADER, ":root {"];

  lines.push(`  /* Colors (dark mode canonical) */`);
  lines.push(...emitVars(PALETTE.base));
  lines.push("");
  lines.push(`  /* Typography */`);
  lines.push(...emitVars(FONTS));
  lines.push("");
  lines.push(`  /* Scale — type, spacing, radii, motion */`);
  lines.push(...emitVars(SCALE));
  lines.push("");
  lines.push(`  color-scheme: dark;`);
  lines.push(`}`);
  lines.push("");

  if (Object.keys(PALETTE.light).length > 0) {
    lines.push(`@media (prefers-color-scheme: light) {`);
    lines.push(`  :root {`);
    lines.push(...emitVars(PALETTE.light, "    "));
    lines.push(`    color-scheme: light;`);
    lines.push(`  }`);
    lines.push(`}`);
    lines.push("");
  }

  return lines.join("\n");
}

// --- File 2: theme-editorial.css (Kumo mappings) ---------------------

/**
 * @param {"text" | "color"} group
 * @param {Record<string, { theme: Record<string, { light: string; dark: string }> }>} tokens
 * @param {string} prefix
 */
function emitGroup(group, tokens, prefix) {
  const lines = [];
  for (const [name, def] of Object.entries(tokens)) {
    const value = def.theme[THEME_NAME]?.light ?? "";
    if (!value) continue;
    lines.push(`  --${prefix}-${name}: ${value};`);
  }
  return lines;
}

function emitMappingsCSS() {
  const lines = [MAPPINGS_HEADER, `[data-theme="${THEME_NAME}"] {`];

  const text = emitGroup("text", EDITORIAL_OVERRIDES.text, "text-color");
  const color = emitGroup("color", EDITORIAL_OVERRIDES.color, "color");

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

// --- Main -----------------------------------------------------------

validate();
await writeFile(PALETTE_OUT, emitPaletteCSS());
console.log(`[theme:gen] wrote ${PALETTE_OUT}`);
await writeFile(MAPPINGS_OUT, emitMappingsCSS());
console.log(`[theme:gen] wrote ${MAPPINGS_OUT}`);
