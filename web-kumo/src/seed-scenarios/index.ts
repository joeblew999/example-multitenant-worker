/**
 * Active seed scenario for THIS build of the frontend.
 *
 * Picked via `VITE_SEED_SCENARIO` env var at build/dev time. Defaults
 * to "editorial". The same env var ALSO sets `<html data-theme="...">`
 * in index.html, so theme + DevAccounts list + backend seed all stay
 * coherent: one env var = one demo.
 *
 * Each scenario module must export a SeedScenario object whose `name`
 * matches the file basename + the SCENARIO env var used by
 * scripts/seed/run.mjs.
 *
 * Add a new scenario:
 *   1. Create src/seed-scenarios/<name>.ts (exports a SeedScenario).
 *   2. Create scripts/seed/scenario.<name>.mjs (creates the users +
 *      orgs the scenario references).
 *   3. Register both in the lookup tables (this file, run.mjs).
 *   4. (Optional) Add a theme with the same name — see KUMO.md §11.
 */

import type { SeedScenario } from "./types";
import { EDITORIAL } from "./editorial";
import { REMYSPORT } from "./remysport";

const SCENARIOS: Record<string, SeedScenario> = {
  editorial: EDITORIAL,
  remysport: REMYSPORT,
};

const requested = import.meta.env.VITE_SEED_SCENARIO ?? "editorial";

export const ACTIVE_SCENARIO: SeedScenario =
  SCENARIOS[requested] ?? EDITORIAL;

export type { SeedScenario, DemoAccount } from "./types";
