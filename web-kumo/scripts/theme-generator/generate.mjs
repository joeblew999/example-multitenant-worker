#!/usr/bin/env node
/**
 * Theme CSS generator — orchestrator.
 *
 * Imports each theme config (config.<name>.mjs), validates against
 * Kumo's THEME_CONFIG (catches typos / removed tokens), and emits
 * the per-theme CSS files via engine.mjs.
 *
 * To add a new theme:
 *   1. Copy config.editorial.mjs → config.<name>.mjs, edit values.
 *   2. Add the import to THEMES below.
 *   3. Run `mise run kumo:theme-gen`.
 *   4. Import the generated palette in src/styles.css.
 *   5. Add the theme name to src/theme.ts THEMES array so the
 *      ThemeToggle can switch to it.
 *
 * Why we hand-roll a generator instead of using Kumo's:
 *   Kumo ships only `dist/scripts/theme-generator/generate-css.d.ts`
 *   (types only) — no .js runtime, not in the package.json exports
 *   map. Their generator is a build-internal helper; consumers must
 *   hand-roll. See KUMO.md §4.
 *
 * Why the emitted CSS is unlayered + scoped to [data-theme="X"]:
 *   - Unlayered beats Kumo's unlayered :root/:host defaults (KUMO.md §3).
 *   - [data-theme="X"] scoping (not :root) lets multiple themes coexist
 *     on the same page. Whichever data-theme is set on <html> wins via
 *     normal CSS variable inheritance.
 */

import { writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

import { emitPaletteCSS, emitMappingsCSS, validate } from "./engine.mjs";
import * as editorial from "./config.editorial.mjs";
import * as remysport from "./config.remysport.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const STYLES_DIR = join(HERE, "..", "..", "src", "styles");

const THEMES = [editorial, remysport];

// --- Run --------------------------------------------------------------

const allWarnings = [];

for (const theme of THEMES) {
  allWarnings.push(...validate(theme));

  const paletteOut = join(STYLES_DIR, `theme-${theme.THEME_NAME}-palette.css`);
  const mappingsOut = join(STYLES_DIR, `theme-${theme.THEME_NAME}.css`);

  await writeFile(paletteOut, emitPaletteCSS(theme));
  console.log(`[theme:gen] wrote ${paletteOut}`);

  await writeFile(mappingsOut, emitMappingsCSS(theme));
  console.log(`[theme:gen] wrote ${mappingsOut}`);
}

if (allWarnings.length) {
  console.warn("[theme:gen] WARN — token names not in kumo base:");
  for (const w of allWarnings) console.warn("  " + w);
}
