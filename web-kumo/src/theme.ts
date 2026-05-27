/**
 * Theme contract for this app.
 *
 *   Editorial is THE theme. Period.
 *
 * Every page — login, signup, dashboard, members, billing, invitations —
 * renders in editorial. That's declared statically by `<html data-theme=
 * "editorial">` in index.html, so the theme is correct from the very
 * first paint (no FOUC, no JS dependency).
 *
 * The `kumo` and `fedramp` themes exist ONLY as a dev-time A/B tool on
 * the /preview showcase, to confirm that components we lift from Kumo
 * still look right when someone re-themes them. The <ThemeToggle> there
 * mutates the <html data-theme> attribute LIVE for the duration of the
 * Preview page, then resets to "editorial" on unmount. No localStorage,
 * no URL param, no persistence — so navigating away always lands you
 * back in editorial, and the toggle can't bleed into another tab or
 * confuse the next page load.
 */

export const THEMES = ["editorial", "kumo", "fedramp"] as const;
export type Theme = (typeof THEMES)[number];

export const DEFAULT_THEME: Theme = "editorial";

/** Live-mutate the <html data-theme> attribute. No persistence. */
export function setHtmlTheme(theme: Theme): void {
  document.documentElement.setAttribute("data-theme", theme);
}
