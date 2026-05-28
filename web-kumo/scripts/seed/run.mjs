#!/usr/bin/env node
/**
 * Seed runner — picks a scenario by env var and drives it.
 *
 * Usage:
 *   SCENARIO=editorial  node scripts/seed/run.mjs        (default)
 *   SCENARIO=remysport  node scripts/seed/run.mjs
 *   BASE=https://...    SCENARIO=remysport node scripts/seed/run.mjs
 *
 * Or via mise:
 *   mise run seed:dev                  (editorial against local :8787)
 *   mise run seed:dev:remysport        (remysport against local :8787)
 *   mise run seed:prod                 (editorial against deployed URL)
 *   mise run seed:prod:remysport       (remysport against deployed URL)
 *
 * Writes a per-scenario manifest at `.seed.<scenario>.json` so multiple
 * scenarios can coexist on disk without clobbering each other.
 *
 * Idempotent — re-running is safe; users/orgs are detected and reused.
 */

import { writeFile } from "node:fs/promises";
import { makeHelpers } from "./helpers.mjs";

const BASE = process.env.BASE ?? "https://localhost:8787";
const PASSWORD = process.env.SEED_PASSWORD ?? "demo-password-123";
const SCENARIO = process.env.SCENARIO ?? "editorial";

const SCENARIOS = {
  editorial:  () => import("./scenario.editorial.mjs"),
  remysport:  () => import("./scenario.remysport.mjs"),
};

async function main() {
  if (!SCENARIOS[SCENARIO]) {
    throw new Error(
      `unknown SCENARIO=${SCENARIO}. Available: ${Object.keys(SCENARIOS).join(", ")}`
    );
  }

  console.log(`[seed] scenario=${SCENARIO}  base=${BASE}`);
  const h = makeHelpers(BASE, PASSWORD);
  await h.healthCheck();

  const scenario = await SCENARIOS[SCENARIO]();
  const result = await scenario.run(h);

  const manifest = {
    scenario: scenario.SCENARIO_NAME,
    description: scenario.DESCRIPTION,
    base: BASE,
    password: PASSWORD,
    ...result,
  };
  const out = `.seed.${SCENARIO}.json`;
  await writeFile(out, JSON.stringify(manifest, null, 2));

  console.log(`\n[seed] done. ${out} written.`);
  console.log(`Password: ${PASSWORD}`);
  console.log("Tour:");
  for (const [u, note] of Object.entries(result.notes)) {
    console.log(`  ${u}  →  ${note}`);
  }
}

main().catch((err) => {
  console.error(`[seed] FAILED: ${err.message}`);
  process.exit(1);
});
