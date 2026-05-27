import { useEffect, useState } from "react";
import { DEFAULT_THEME, setHtmlTheme, THEMES, type Theme } from "../theme";

/**
 * Dev-only theme A/B tool. Mounted ONLY on the /preview page.
 *
 * Mutates <html data-theme> live so you can verify components paint
 * correctly under editorial / kumo / fedramp. Resets to editorial on
 * unmount, so navigating away always lands you back in the canonical
 * theme — there is no persistence (no localStorage, no URL param).
 *
 * This is intentional: editorial is THE theme for the app. The toggle
 * is a developer affordance, not a user preference.
 */
export function ThemeToggle() {
  const [theme, setTheme] = useState<Theme>(DEFAULT_THEME);

  useEffect(() => {
    setHtmlTheme(theme);
  }, [theme]);

  // Reset to editorial when the Preview page (and this component) unmounts.
  useEffect(() => {
    return () => {
      setHtmlTheme(DEFAULT_THEME);
    };
  }, []);

  return (
    <div
      role="group"
      aria-label="Theme"
      style={{
        display: "inline-flex",
        gap: 2,
        padding: 2,
        border: "1px solid var(--border-visible)",
        borderRadius: "var(--radius-pill)",
        fontFamily: "var(--font-mono)",
        fontSize: 11,
        letterSpacing: "0.08em",
        textTransform: "uppercase",
      }}
    >
      {THEMES.map((t) => {
        const active = t === theme;
        return (
          <button
            key={t}
            type="button"
            onClick={() => setTheme(t)}
            aria-pressed={active}
            style={{
              all: "unset",
              cursor: "pointer",
              padding: "2px 10px",
              borderRadius: "var(--radius-pill)",
              background: active ? "var(--accent)" : "transparent",
              color: active ? "var(--text-display)" : "var(--text-secondary)",
              minHeight: 0,
            }}
          >
            {t}
          </button>
        );
      })}
    </div>
  );
}
