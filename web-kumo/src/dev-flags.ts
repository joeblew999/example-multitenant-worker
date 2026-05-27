/**
 * Dev-only feature flags. Single source of truth so multiple components
 * gate behind the same env var without drift.
 *
 * Set `VITE_SHOW_TEST_ACCOUNTS=false` in .env.production (or wherever
 * your build environment overrides Vite env) to hide the DevAccounts
 * card AND restore the production sign-out flow (-> /login).
 */

export function devAccountsEnabled(): boolean {
  const flag = (import.meta as { env?: { VITE_SHOW_TEST_ACCOUNTS?: string } }).env?.VITE_SHOW_TEST_ACCOUNTS;
  if (flag === "false") return false;
  return true;
}
