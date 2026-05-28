#!/usr/bin/env node
/**
 * Theme CSS generator — auto-discovers scenarios.
 *
 * Globs `scenarios/* /scenario.mjs` (one folder per scenario), validates
 * each against Kumo's THEME_CONFIG, and emits per-scenario palette +
 * Kumo-mappings CSS files.
 *
 * To add a new scenario: create `scenarios/<name>/scenario.mjs` exporting
 * SCENARIO_NAME + THEME (see src/seed-scenarios/types.ts). Run
 * `mise run kumo:theme-gen`. No registry to update.
 *
 * Why hand-roll instead of Kumo's generator: see KUMO.md §4.
 */

import { readdir, writeFile, stat } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

import { emitPaletteCSS, emitMappingsCSS, validate } from "./engine.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..", "..");
const SCENARIOS_DIR = join(ROOT, "scenarios");
const STYLES_DIR = join(ROOT, "src", "styles");

/** Discover every scenarios/<name>/scenario.mjs. */
async function discover() {
  const entries = await readdir(SCENARIOS_DIR);
  const scenarios = [];
  for (const name of entries) {
    const file = join(SCENARIOS_DIR, name, "scenario.mjs");
    try {
      await stat(file);
    } catch {
      continue;
    }
    const mod = await import(file);
    if (!mod.SCENARIO_NAME || !mod.THEME) {
      console.warn(`[theme:gen] skipping ${name}: missing SCENARIO_NAME or THEME`);
      continue;
    }
    if (mod.SCENARIO_NAME !== name) {
      throw new Error(
        `scenarios/${name}/scenario.mjs has SCENARIO_NAME="${mod.SCENARIO_NAME}" — must equal folder name`
      );
    }
    scenarios.push(mod);
  }
  return scenarios;
}

const scenarios = await discover();
if (!scenarios.length) {
  console.error(`[theme:gen] no scenarios found under ${SCENARIOS_DIR}`);
  process.exit(1);
}

const warnings = [];
for (const scenario of scenarios) {
  warnings.push(...validate(scenario));

  const paletteOut = join(STYLES_DIR, `theme-${scenario.SCENARIO_NAME}-palette.css`);
  const mappingsOut = join(STYLES_DIR, `theme-${scenario.SCENARIO_NAME}.css`);

  await writeFile(paletteOut, emitPaletteCSS(scenario));
  console.log(`[theme:gen] wrote ${paletteOut}`);
  await writeFile(mappingsOut, emitMappingsCSS(scenario));
  console.log(`[theme:gen] wrote ${mappingsOut}`);
}

if (warnings.length) {
  console.warn("[theme:gen] WARN — token names not in kumo base:");
  for (const w of warnings) console.warn("  " + w);
}

console.log(`[theme:gen] ${scenarios.length} scenario(s): ${scenarios.map(s => s.SCENARIO_NAME).join(", ")}`);
