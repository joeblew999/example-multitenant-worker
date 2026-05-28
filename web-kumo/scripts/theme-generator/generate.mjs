#!/usr/bin/env node
/**
 * Editorial theme — CSS generator.
 *
 * Reads ./config.editorial.mjs and emits src/styles/theme-editorial.css.
 * Output shape mirrors Kumo's own theme-fedramp.css: an @layer base
 * block scoped to [data-theme="editorial"], so it slots into Kumo's
 * existing theme-switching mechanism (and our ThemeToggle).
 *
 * Also cross-checks token names against Kumo's THEME_CONFIG and warns
 * about typos / tokens that don't exist in the kumo base.
 */

import { writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

import { THEME_CONFIG as KUMO_CONFIG } from "@cloudflare/kumo/scripts/theme-generator/config";
import { THEME_NAME, EDITORIAL_OVERRIDES } from "./config.editorial.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT_PATH = join(HERE, "..", "..", "src", "styles", "theme-editorial.css");

const HEADER = `/**
 * AUTO-GENERATED — do not edit directly.
 *
 * Source:    scripts/theme-generator/config.editorial.mjs
 * Generator: scripts/theme-generator/generate.mjs
 * Regen:     mise run kumo:theme-gen   (or pnpm theme:gen)
 *
 * UNLAYERED on purpose. Kumo emits an unlayered \`:root, :host\` rule
 * that sets every \`--color-kumo-*\` token to its Kumo-default value
 * (e.g. brand → blue via \`light-dark(oklch(...), oklch(...))\`). Per
 * the cascade-layers spec, unlayered rules ALWAYS beat layered ones
 * regardless of selector specificity — so wrapping our overrides in
 * \`@layer base\` made the editorial brand red silently lose to Kumo's
 * unlayered blue. Keeping these rules unlayered lets them tie on
 * layer (both none) and win on specificity + source order.
 *
 * Values reference vars from legacy-styles.css which handles light/dark
 * via prefers-color-scheme.
 */
`;

function validate() {
  const warnings = [];
  for (const group of ["text", "color"]) {
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

function emitGroup(group, tokens, prefix) {
  const lines = [];
  for (const [name, def] of Object.entries(tokens)) {
    const value = def.theme[THEME_NAME]?.light ?? "";
    if (!value) continue;
    lines.push(`    --${prefix}-${name}: ${value};`);
  }
  return lines;
}

function emit() {
  const lines = [];
  lines.push(HEADER);
  lines.push(`[data-theme="${THEME_NAME}"] {`);

  const text = emitGroup("text", EDITORIAL_OVERRIDES.text, "text-color").map((l) => l.slice(2));
  const color = emitGroup("color", EDITORIAL_OVERRIDES.color, "color").map((l) => l.slice(2));

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
