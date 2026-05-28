/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** When "false", DevAccounts hides + sign-out reverts to /login. Default: visible. */
  readonly VITE_SHOW_TEST_ACCOUNTS?: "true" | "false";
  /**
   * Active demo scenario. Picks the DevAccounts list (src/seed-scenarios/<name>.ts)
   * AND the canonical theme (via index.html's `<html data-theme="%VITE_SEED_SCENARIO%">`).
   * Backend seed picks the matching scripts/seed/scenario.<name>.mjs.
   * Default: "editorial".
   */
  readonly VITE_SEED_SCENARIO?: "editorial" | "remysport";
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
