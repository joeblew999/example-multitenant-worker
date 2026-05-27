/**
 * Dev-only feature flags. Single source of truth so multiple components
 * gate behind the same env var without drift.
 *
 * Set `VITE_SHOW_TEST_ACCOUNTS=false` in `.env.production` (or whatever
 * your build env uses) to hide the DevAccounts card AND restore the
 * production sign-out flow (→ /login). Default: visible.
 *
 * Env var type is declared in `vite-env.d.ts`.
 */

export const devAccountsEnabled = (): boolean =>
  import.meta.env.VITE_SHOW_TEST_ACCOUNTS !== "false";
