#!/usr/bin/env node
/**
 * Seed runner — auto-discovers scenarios, picks one by env var.
 *
 * Usage:
 *   SCENARIO=editorial  node scripts/seed/run.mjs   (default)
 *   SCENARIO=remysport  node scripts/seed/run.mjs
 *   BASE=https://...    SCENARIO=remysport node scripts/seed/run.mjs
 *
 * Or via mise:
 *   mise run seed:dev                (editorial against local :8787)
 *   mise run seed:dev:remysport      (remysport against local :8787)
 *   mise run seed:prod               (editorial against deployed)
 *   mise run seed:prod:remysport     (remysport against deployed)
 *
 * Each scenario lives in scenarios/<name>/scenario.mjs and exports
 * SCENARIO_NAME + seed(helpers). No scenario registry to maintain;
 * dropping a new folder is the only step.
 *
 * Writes a per-scenario manifest at `.seed.<scenario>.json` so multiple
 * scenarios coexist on disk without clobbering each other.
 *
 * Idempotent — re-running is safe; users/orgs are detected and reused.
 */

import { readdir, writeFile, stat } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { makeHelpers } from "./helpers.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "..", "..");
const SCENARIOS_DIR = join(ROOT, "scenarios");

const BASE = process.env.BASE ?? "https://localhost:8787";
const SCENARIO = process.env.SCENARIO ?? "editorial";

async function listScenarios() {
  const out = [];
  for (const name of await readdir(SCENARIOS_DIR)) {
    const file = join(SCENARIOS_DIR, name, "scenario.mjs");
    try {
      await stat(file);
      out.push(name);
    } catch {}
  }
  return out;
}

async function loadScenario(name) {
  const file = join(SCENARIOS_DIR, name, "scenario.mjs");
  try {
    await stat(file);
  } catch {
    const available = await listScenarios();
    throw new Error(
      `unknown SCENARIO=${name}. Available: ${available.join(", ") || "(none)"}`
    );
  }
  return await import(file);
}

async function main() {
  const scenario = await loadScenario(SCENARIO);
  console.log(`[seed] scenario=${SCENARIO}  base=${BASE}`);
  console.log(`[seed] ${scenario.DESCRIPTION}`);

  const h = makeHelpers(BASE, scenario.PASSWORD);
  await h.healthCheck();

  const result = await scenario.seed(h);

  const manifest = {
    scenario: scenario.SCENARIO_NAME,
    description: scenario.DESCRIPTION,
    base: BASE,
    password: scenario.PASSWORD,
    ...result,
  };
  const out = `.seed.${SCENARIO}.json`;
  await writeFile(out, JSON.stringify(manifest, null, 2));

  console.log(`\n[seed] done. ${out} written.`);
  console.log(`Password: ${scenario.PASSWORD}`);
  console.log("Tour:");
  for (const [u, note] of Object.entries(result.notes)) {
    console.log(`  ${u}  →  ${note}`);
  }
}

main().catch((err) => {
  console.error(`[seed] FAILED: ${err.message}`);
  process.exit(1);
});
