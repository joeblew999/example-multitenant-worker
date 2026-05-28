/**
 * Active demo scenario for this build of the frontend.
 *
 * Auto-discovers every `scenarios/<name>/scenario.mjs` via Vite's
 * `import.meta.glob`. No SCENARIOS map to maintain — drop a new
 * scenarios/<name>/ folder and it's picked up automatically.
 *
 * The active scenario is picked by `VITE_SEED_SCENARIO` (default:
 * "editorial"). The same env var ALSO sets `<html data-theme="...">`
 * in index.html, so theme + DevAccounts list + backend seed all stay
 * coherent: one env var = one demo.
 */

import type { Scenario } from "./types";

// eager: true so ACTIVE_SCENARIO is synchronous. Cost: every scenario's
// metadata (theme tokens + accounts) lives in the bundle. Code-split via
// `eager: false` + dynamic await if/when bundle size matters.
const modules = import.meta.glob<Scenario>(
  "/scenarios/*/scenario.mjs",
  { eager: true },
);

const SCENARIOS: Record<string, Scenario> = Object.fromEntries(
  Object.values(modules).map((mod) => [mod.SCENARIO_NAME, mod]),
);

const requested = import.meta.env.VITE_SEED_SCENARIO ?? "editorial";

export const ACTIVE_SCENARIO: Scenario =
  SCENARIOS[requested] ?? SCENARIOS.editorial;

/** All discovered scenario names — useful for diagnostics or a switcher UI. */
export const AVAILABLE_SCENARIOS: string[] = Object.keys(SCENARIOS).sort();

export type { Scenario, DemoAccount } from "./types";
