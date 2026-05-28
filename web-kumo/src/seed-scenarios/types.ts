import type { Theme } from "../theme";

/**
 * One sign-in card on the DevAccounts panel.
 *
 * Each scenario file exports an array of these. The backend seed
 * script (scripts/seed/scenario.<name>.mjs) creates the matching
 * users; the strings here MUST stay in sync with that file.
 */
export interface DemoAccount {
  email: string;
  /** Display name shown on the badge (e.g. "Alice (owner of 6 orgs)"). */
  label: string;
  /** One-line description of what makes this account interesting. */
  scenario: string;
  /** Path to navigate to on successful login. */
  landAt: string;
  /** Kumo Badge variant for the chip. */
  badge: "orange" | "blue" | "purple" | "teal";
}

export interface SeedScenario {
  /** Canonical name — matches the SCENARIO env var + scripts/seed/scenario.<name>.mjs. */
  name: string;
  /** Theme that pairs with this scenario by convention. */
  canonicalTheme: Theme;
  /** Shared password for every demo account in this scenario. */
  password: string;
  /** Sign-in cards rendered by DevAccounts. */
  accounts: DemoAccount[];
}
