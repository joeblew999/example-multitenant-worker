/**
 * Shape of a scenario module — every `scenarios/<name>/scenario.mjs`
 * exports values matching this interface. Used by:
 *   - Frontend: src/seed-scenarios/index.ts (auto-discovery via import.meta.glob)
 *   - Theme gen: scripts/theme-generator/engine.mjs (reads THEME)
 *   - Backend seed: scripts/seed/run.mjs (calls seed(helpers))
 *
 * Folder name MUST equal SCENARIO_NAME so `<html data-theme="...">`
 * resolves the right palette CSS file.
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

/** Scenario module surface. */
export interface Scenario {
  /** Must equal the folder name. */
  readonly SCENARIO_NAME: string;
  readonly DESCRIPTION: string;
  readonly PASSWORD: string;
  /** Frontend DevAccounts panel entries. */
  readonly ACCOUNTS: DemoAccount[];
  /** Theme tokens consumed by scripts/theme-generator/engine.mjs. */
  readonly THEME: {
    colorScheme?: "dark" | "light";
    colorSchemeLight?: "dark" | "light";
    palette: {
      base: Record<string, string>;
      /** Sparse light-mode override; vars not listed inherit from base. */
      light: Record<string, string>;
    };
    fonts: Record<string, string>;
    scale: Record<string, string>;
    kumoOverrides: {
      text: Record<string, string>;
      color: Record<string, string>;
    };
  };
  /** Backend seed function — not consumed by frontend. */
  readonly seed: (helpers: unknown) => Promise<unknown>;
}
