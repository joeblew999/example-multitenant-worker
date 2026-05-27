import { useEffect, useState } from "react";

const STORAGE_KEY = "wm.theme";
const THEMES = ["editorial", "kumo", "fedramp"] as const;
type Theme = (typeof THEMES)[number];

function isTheme(v: unknown): v is Theme {
  return typeof v === "string" && (THEMES as readonly string[]).includes(v);
}

function readInitial(): Theme {
  const fromUrl = new URLSearchParams(window.location.search).get("theme");
  if (isTheme(fromUrl)) return fromUrl;
  const stored = localStorage.getItem(STORAGE_KEY);
  if (isTheme(stored)) return stored;
  return "editorial";
}

function apply(theme: Theme) {
  document.documentElement.setAttribute("data-theme", theme);
}

export function ThemeToggle() {
  const [theme, setTheme] = useState<Theme>(readInitial);

  useEffect(() => {
    apply(theme);
    localStorage.setItem(STORAGE_KEY, theme);
  }, [theme]);

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
