/**
 * RemySports theme — config.
 *
 * Branding derived from the remy-sport-biz pitchdeck slideshow
 * (joeblew999/remy-sport-biz/pitchdeck/index.html). Paper-bg light
 * theme, warm Thai-marigold orange accent, Inter + Space Grotesk +
 * IBM Plex Mono.
 *
 * Structurally identical to config.editorial.mjs — same exports, same
 * shape — proving the multi-theme architecture. Add new themes by
 * copying this file, swapping THEME_NAME + values, registering in
 * generate.mjs's THEMES array.
 *
 * @typedef {import("@cloudflare/kumo/scripts/theme-generator/types").ThemeConfig} ThemeConfig
 */

export const THEME_NAME = "remysport";

// ============================================================
// PALETTE — paper-bg light theme. No prefers-color-scheme variant
// today (the brand IS light). Add one later if a dark mode is needed.
// ============================================================

export const PALETTE = {
  colorScheme: "light",
  base: {
    // Surfaces — warm paper tones (oklch hue ~80–90)
    "paper":           "oklch(0.985 0.005 90)",
    "paper-2":         "oklch(0.965 0.008 90)",
    "paper-3":         "oklch(0.93 0.01 80)",
    "rule":            "oklch(0.85 0.005 90)",

    // Map onto the editorial-shaped var names so editorial-chrome.css
    // (.shell/.topbar/.brand/.page) renders correctly under remysport
    // without per-chrome forks. The chrome is theme-agnostic; the var
    // values do the lifting.
    "black":           "oklch(0.985 0.005 90)",   // page background
    "surface":         "oklch(0.985 0.005 90)",   // = paper
    "surface-raised":  "oklch(0.965 0.008 90)",   // = paper-2
    "border":          "oklch(0.85 0.005 90)",    // = rule
    "border-visible":  "oklch(0.78 0.01 80)",     // slightly darker rule

    // Ink — cool dark blue-gray (oklch hue ~270)
    "text-disabled":   "oklch(0.65 0.005 270)",
    "text-secondary":  "oklch(0.55 0.01 270)",
    "text-primary":    "oklch(0.32 0.01 270)",
    "text-display":    "oklch(0.18 0.01 270)",

    // Accent — warm Thai marigold
    "accent":          "oklch(0.62 0.18 35)",
    "accent-subtle":   "oklch(0.62 0.18 35 / 0.15)",

    // Status — same hue family as accent but mapped to semantic intent
    "success":         "oklch(0.55 0.14 145)",    // muted green
    "warning":         "oklch(0.75 0.16 75)",     // amber
    "error":           "oklch(0.55 0.22 25)",     // live-red (matches pitchdeck --live)
    "interactive":     "oklch(0.48 0.18 35)",     // = accent-deep
  },
  // No light override — the base IS light. (Dark mode for remysport
  // would need its own palette; not in scope today.)
  light: {},
};

// ============================================================
// FONTS — Inter body, Space Grotesk display, IBM Plex Mono.
//
// font-display maps to Space Grotesk so the same h1.font-heading rule
// in theme-editorial-extras.css picks up the right display font under
// remysport without per-theme forks (Kumo doesn't expose font-family
// as a token, so the override is selector-based — KUMO.md §6).
//
// Noto Sans Thai isn't a stack we map by default; pages that need it
// can opt in via direct font-family.
// ============================================================

export const FONTS = {
  "font-display": '"Space Grotesk", "DM Sans", system-ui, sans-serif',
  "font-body":    '"Inter", system-ui, sans-serif',
  "font-mono":    '"IBM Plex Mono", "JetBrains Mono", "SF Mono", monospace',
};

// ============================================================
// SCALE — identical to editorial. Most projects can share scale; only
// override here if remysport actually needs different sizing/spacing.
// ============================================================

export const SCALE = {
  "display-xl":  "72px",
  "display-lg":  "48px",
  "display-md":  "36px",
  "heading":     "24px",
  "subheading":  "18px",
  "body":        "16px",
  "body-sm":     "14px",
  "caption":     "12px",
  "label":       "11px",

  "space-2xs":   "2px",
  "space-xs":    "4px",
  "space-sm":    "8px",
  "space-md":    "16px",
  "space-lg":    "24px",
  "space-xl":    "32px",
  "space-2xl":   "48px",
  "space-3xl":   "64px",
  "space-4xl":   "96px",

  "radius-card":    "16px",
  "radius-compact": "8px",
  "radius-tech":    "4px",
  "radius-pill":    "999px",

  "motion-fast":  "150ms",
  "motion-base":  "250ms",
  "easing":       "cubic-bezier(0.25, 0.1, 0.25, 1)",
};

// ============================================================
// KUMO_OVERRIDES — same mapping structure as editorial. var() refs
// resolve against THIS theme's palette at runtime.
// ============================================================

const accent = "var(--accent)";
const v = (value) => ({ theme: { [THEME_NAME]: { light: value, dark: value } } });

export const KUMO_OVERRIDES = {
  text: {
    "kumo-default":     v("var(--text-primary)"),
    "kumo-strong":      v("var(--text-display)"),
    "kumo-subtle":      v("var(--text-secondary)"),
    "kumo-inactive":    v("var(--text-disabled)"),
    "kumo-placeholder": v("var(--text-disabled)"),
    "kumo-brand":       v(accent),
    "kumo-link":        v(accent),
  },
  color: {
    "kumo-canvas":      v("var(--surface)"),
    "kumo-elevated":    v("var(--surface-raised)"),
    "kumo-recessed":    v("var(--surface)"),
    "kumo-base":        v("var(--surface-raised)"),
    "kumo-tint":        v("var(--surface-raised)"),
    "kumo-overlay":     v("var(--surface-raised)"),
    "kumo-control":     v("var(--surface-raised)"),
    "kumo-fill":        v("var(--surface-raised)"),
    "kumo-fill-hover":  v("var(--border)"),
    "kumo-interact":    v("var(--border-visible)"),
    "kumo-contrast":    v("var(--text-display)"),
    "kumo-line":        v("var(--border)"),
    "kumo-hairline":    v("var(--border-visible)"),
    "kumo-focus":       v(accent),
    "kumo-brand":       v(accent),
    "kumo-brand-hover": v("var(--interactive)"),   // accent-deep on hover
    "kumo-shadow-edge": v("transparent"),
    "kumo-shadow-drop": v("transparent"),
    "kumo-info":          v("oklch(70% 0.10 240)"),
    "kumo-info-tint":     v("oklch(70% 0.10 240)"),
    "kumo-success":       v("var(--success)"),
    "kumo-success-tint":  v("var(--success)"),
    "kumo-warning":       v("var(--warning)"),
    "kumo-warning-tint":  v("var(--warning)"),
    "kumo-danger":        v("var(--error)"),
    "kumo-danger-tint":   v("var(--error)"),
  },
  typography: {},
};
