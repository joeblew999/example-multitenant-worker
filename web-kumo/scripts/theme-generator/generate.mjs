#!/usr/bin/env node
/**
 * Editorial theme — CSS generator.
 *
 * Reads ./config.editorial.mjs and emits src/styles/theme-editorial.css.
 *
 * ============================================================
 * Why this exists instead of calling Kumo's generator directly
 * ============================================================
 *
 * Kumo's `scripts/theme-generator/generate-css.ts` exports a
 * `generateThemeOverrideCSS(config, themeName)` function that does
 * almost exactly what this script does. We CANNOT use it from a
 * consumer project:
 *
 *   1. Kumo's `dist/scripts/theme-generator/generate-css.d.ts` ships
 *      only the type defs — there is no `generate-css.js` runtime.
 *   2. The package.json `exports` map only exposes
 *      `./scripts/theme-generator/{config,types}`. Deep imports past
 *      that fail under strict ESM resolution.
 *   3. Kumo invokes the generator via `tsx scripts/theme-generator/index.ts`
 *      from inside their own monorepo — that path is build-internal.
 *
 * So adding a new theme means hand-rolling a generator like this one.
 * If/when Kumo publishes the generator as part of their public API,
 * this file becomes a one-line wrapper around their function.
 *
 * What we DO use from Kumo: the typed `THEME_CONFIG` (for validating
 * that our token names aren't typos) and the `ThemeConfig`-family
 * types (via JSDoc) for editor support.
 *
 * ============================================================
 * Why the output is UNLAYERED (no @layer wrapper)
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
 * silently lose. Verified by switching to `<html data-theme="fedramp">`
 * in a stock Kumo dev server — `--color-kumo-brand` still paints
 * Kumo blue, not fedramp's intended color.
 *
 * We emit `[data-theme="editorial"] { ... }` unlayered so it ties
 * Kumo's `:root, :host` on layer (both none) and wins on specificity
 * (0,1,0 > :root's 0,0,1) + source order.
 */

import { writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

import { THEME_CONFIG as KUMO_CONFIG } from "@cloudflare/kumo/scripts/theme-generator/config";
import { THEME_NAME, EDITORIAL_OVERRIDES } from "./config.editorial.mjs";

/**
 * @typedef {import("@cloudflare/kumo/scripts/theme-generator/types").ThemeConfig} ThemeConfig
 * @typedef {import("@cloudflare/kumo/scripts/theme-generator/types").TokenDefinition} TokenDefinition
 * @typedef {import("@cloudflare/kumo/scripts/theme-generator/types").TextTokens} TextTokens
 * @typedef {import("@cloudflare/kumo/scripts/theme-generator/types").ColorTokens} ColorTokens
 */

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT_PATH = join(HERE, "..", "..", "src", "styles", "theme-editorial.css");

const HEADER = `/**
 * AUTO-GENERATED — do not edit directly.
 *
 * Source:    scripts/theme-generator/config.editorial.mjs
 * Generator: scripts/theme-generator/generate.mjs
 * Regen:     mise run kumo:theme-gen   (or pnpm theme:gen)
 *
 * UNLAYERED on purpose — see generator header for why \`@layer base\`
 * would let Kumo's unlayered \`:root, :host\` defaults silently win.
 * Values reference vars from legacy-styles.css which handles light/dark
 * via prefers-color-scheme.
 */
`;

/**
 * Cross-check our token names against Kumo's THEME_CONFIG. Catches
 * typos and tokens that have been renamed/removed upstream.
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

/**
 * @param {"text" | "color"} group
 * @param {Record<string, { theme: Record<string, { light: string; dark: string }> }>} tokens
 * @param {string} prefix
 * @returns {string[]}
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

function emit() {
  const lines = [HEADER, `[data-theme="${THEME_NAME}"] {`];

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

validate();
await writeFile(OUT_PATH, emit());
console.log(`[theme:gen] wrote ${OUT_PATH}`);
