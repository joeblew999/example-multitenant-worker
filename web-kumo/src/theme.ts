/**
 * Theme bootstrap. Runs at app boot from main.tsx so the chosen theme
 * is applied to <html> regardless of which page the user landed on.
 *
 * Without this, only /preview (where <ThemeToggle> is mounted) would
 * restore the user's saved choice — every other page (login, dashboard,
 * etc.) would show whatever theme index.html declares by default.
 *
 * The ThemeToggle component reads + writes the same source of truth.
 */

const STORAGE_KEY = "wm.theme";

export const THEMES = ["editorial", "kumo", "fedramp"] as const;
export type Theme = (typeof THEMES)[number];

export const THEME_STORAGE_KEY = STORAGE_KEY;

function isTheme(v: unknown): v is Theme {
  return typeof v === "string" && (THEMES as readonly string[]).includes(v);
}

/** URL `?theme=` wins; then localStorage; default `editorial`. */
export function readInitialTheme(): Theme {
  const fromUrl = new URLSearchParams(window.location.search).get("theme");
  if (isTheme(fromUrl)) return fromUrl;
  const stored = localStorage.getItem(STORAGE_KEY);
  if (isTheme(stored)) return stored;
  return "editorial";
}

export function applyTheme(theme: Theme): void {
  document.documentElement.setAttribute("data-theme", theme);
  localStorage.setItem(STORAGE_KEY, theme);
}
