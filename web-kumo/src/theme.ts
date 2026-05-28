/**
 * Theme contract for this app.
 *
 * Each demo scenario (scenarios/<name>/scenario.mjs) declares its own
 * theme via the THEME export. The canonical theme for THIS build is
 * picked via VITE_SEED_SCENARIO and declared on `<html data-theme>` by
 * index.html's `%VITE_SEED_SCENARIO%` substitution — paints on first
 * frame with no JS dependency.
 *
 * The Kumo built-in themes (kumo, fedramp) are always available too,
 * so the /preview ThemeToggle can A/B every component under each
 * style. Scenario themes get listed first, then Kumo's built-ins.
 */

import { AVAILABLE_SCENARIOS } from "./seed-scenarios";

/** Kumo ships these two themes; they coexist with our scenario themes. */
const KUMO_BUILTIN_THEMES = ["kumo", "fedramp"] as const;

export const THEMES: readonly string[] = [
  ...AVAILABLE_SCENARIOS,
  ...KUMO_BUILTIN_THEMES,
] as const;

export type Theme = string;

/** Live-mutate the <html data-theme> attribute. No persistence. */
export function setHtmlTheme(theme: Theme): void {
  document.documentElement.setAttribute("data-theme", theme);
}
