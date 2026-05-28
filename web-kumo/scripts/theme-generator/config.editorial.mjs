/**
 * Editorial theme — config (single source of truth for this theme).
 *
 * The generator (`generate.mjs`) emits two CSS files from this:
 *
 *   1. theme-editorial-palette.css — [data-theme="editorial"] vars
 *      (colors, fonts, scale) + a `prefers-color-scheme: light` override.
 *      Every other CSS file references these via `var(--accent)` etc.
 *
 *   2. theme-editorial.css — [data-theme="editorial"] block mapping
 *      Kumo's `--color-kumo-*` tokens to the palette via var() refs.
 *
 * To add a new theme, copy this file as `config.<name>.mjs`, swap the
 * THEME_NAME + values, register it in `generate.mjs`'s THEMES array.
 *
 * @typedef {import("@cloudflare/kumo/scripts/theme-generator/types").ThemeConfig} ThemeConfig
 */

export const THEME_NAME = "editorial";

// ============================================================
// PALETTE — canonical hex values, base (dark) + light overrides.
//
// Keys are kebab-case so they're emitted as `--<key>: <value>` 1:1.
// `base` is the default; `light` is a sparse override — only list the
// vars that change for light mode (anything omitted inherits from base).
// `colorScheme` / `colorSchemeLight` set the CSS color-scheme property
// so native form controls and scrollbars match.
// ============================================================

export const PALETTE = {
  colorScheme: "dark",
  colorSchemeLight: "light",
  base: {
    "black":           "#000000",
    "surface":         "#111111",
    "surface-raised":  "#1a1a1a",
    "border":          "#222222",
    "border-visible":  "#333333",
    "text-disabled":   "#666666",
    "text-secondary":  "#999999",
    "text-primary":    "#e8e8e8",
    "text-display":    "#ffffff",
    "accent":          "#d71921",
    "accent-subtle":   "rgba(215, 25, 33, 0.15)",
    "success":         "#4a9e5c",
    "warning":         "#d4a843",
    "error":           "#d71921",
    "interactive":     "#5b9bf6",
  },
  light: {
    "black":           "#f5f5f5",
    "surface":         "#ffffff",
    "surface-raised":  "#f0f0f0",
    "border":          "#e8e8e8",
    "border-visible":  "#cccccc",
    "text-disabled":   "#999999",
    "text-secondary":  "#666666",
    "text-primary":    "#1a1a1a",
    "text-display":    "#000000",
    "accent-subtle":   "rgba(215, 25, 33, 0.08)",
    "interactive":     "#007aff",
  },
};

// ============================================================
// FONTS — font-family stacks.
// ============================================================

export const FONTS = {
  "font-display": '"Doto", "Space Mono", monospace',
  "font-body":    '"Space Grotesk", "DM Sans", system-ui, sans-serif',
  "font-mono":    '"Space Mono", "JetBrains Mono", "SF Mono", monospace',
};

// ============================================================
// SCALE — type sizes, spacing, radii, motion.
// ============================================================

export const SCALE = {
  // Type scale
  "display-xl":  "72px",
  "display-lg":  "48px",
  "display-md":  "36px",
  "heading":     "24px",
  "subheading":  "18px",
  "body":        "16px",
  "body-sm":     "14px",
  "caption":     "12px",
  "label":       "11px",

  // Spacing scale (8px base)
  "space-2xs":   "2px",
  "space-xs":    "4px",
  "space-sm":    "8px",
  "space-md":    "16px",
  "space-lg":    "24px",
  "space-xl":    "32px",
  "space-2xl":   "48px",
  "space-3xl":   "64px",
  "space-4xl":   "96px",

  // Radii
  "radius-card":    "16px",
  "radius-compact": "8px",
  "radius-tech":    "4px",
  "radius-pill":    "999px",

  // Motion
  "motion-fast":  "150ms",
  "motion-base":  "250ms",
  "easing":       "cubic-bezier(0.25, 0.1, 0.25, 1)",
};

// ============================================================
// KUMO_OVERRIDES — Kumo `--color-kumo-*` token mappings.
// References the vars defined in PALETTE/FONTS/SCALE via `var(--...)`.
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
    "kumo-brand-hover": v(accent),
    "kumo-shadow-edge": v("transparent"),
    "kumo-shadow-drop": v("transparent"),
    "kumo-info":          v("oklch(70% 0.10 240)"),
    "kumo-info-tint":     v("oklch(70% 0.10 240)"),
    "kumo-success":       v("oklch(70% 0.13 150)"),
    "kumo-success-tint":  v("oklch(70% 0.13 150)"),
    "kumo-warning":       v("oklch(80% 0.16 80)"),
    "kumo-warning-tint":  v("oklch(80% 0.16 80)"),
    "kumo-danger":        v(accent),
    "kumo-danger-tint":   v(accent),
  },
  typography: {},
};
