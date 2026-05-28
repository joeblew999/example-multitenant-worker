import { useEffect, useState } from "react";
import { setHtmlTheme, THEMES, type Theme } from "../theme";

/**
 * Dev-only theme A/B tool. Mounted ONLY on the /preview page.
 *
 * Mutates <html data-theme> live so you can verify components paint
 * correctly under every available theme. Restores whatever theme was
 * canonical when the toggle mounted on unmount — so navigating away
 * always lands you back in the demo's canonical theme (set at build
 * time via VITE_SEED_SCENARIO).
 *
 * No persistence (no localStorage, no URL param). The toggle is a
 * developer affordance, not a user preference.
 */

/** Read the current <html data-theme> attribute (the canonical scenario). */
function readHtmlTheme(): Theme {
  const t = document.documentElement.getAttribute("data-theme");
  if (t && (THEMES as readonly string[]).includes(t)) return t as Theme;
  // Fallback if the canonical theme isn't in THEMES (shouldn't happen
  // when scenario name === theme name by convention, but defensive).
  return THEMES[0];
}

export function ThemeToggle() {
  // Snapshot the canonical theme once at mount. This is what we restore
  // to on unmount, regardless of what the user toggled to mid-page.
  const [canonical] = useState<Theme>(readHtmlTheme);
  const [theme, setTheme] = useState<Theme>(canonical);

  useEffect(() => {
    setHtmlTheme(theme);
  }, [theme]);

  useEffect(() => {
    return () => {
      setHtmlTheme(canonical);
    };
  }, [canonical]);

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
