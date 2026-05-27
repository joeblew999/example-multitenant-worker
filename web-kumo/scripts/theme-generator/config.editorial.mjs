/**
 * Editorial theme — config.
 *
 * Matches the shape of @cloudflare/kumo's THEME_CONFIG (text + color +
 * typography token groups) but only declares tokens that DIFFER from
 * default kumo. Everything else falls through to theme-kumo.css.
 *
 * Token values reference CSS custom properties from legacy-styles.css
 * (`--surface`, `--accent`, etc.) rather than literal hex — legacy is
 * the canonical palette (byte-identical with example-multitenant-worker/web/),
 * and uses `prefers-color-scheme` to switch light/dark itself. So we
 * emit single-value tokens, not light-dark() — the variables handle
 * mode switching one level up.
 *
 * @typedef {import("@cloudflare/kumo/scripts/theme-generator/types").ThemeConfig} ThemeConfig
 */

const accent = "var(--accent)";

/** Single-value entry — used when light/dark are both the same expression. */
const v = (value) => ({ theme: { editorial: { light: value, dark: value } } });

export const THEME_NAME = "editorial";

export const EDITORIAL_OVERRIDES = {
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
    // Surfaces
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

    // Borders + focus
    "kumo-line":        v("var(--border)"),
    "kumo-hairline":    v("var(--border-visible)"),
    "kumo-focus":       v(accent),

    // Brand
    "kumo-brand":       v(accent),
    "kumo-brand-hover": v(accent),

    // Shadows — editorial avoids drop shadows
    "kumo-shadow-edge": v("transparent"),
    "kumo-shadow-drop": v("transparent"),

    // Status tints — distinct enough for Banner intents to read at a glance.
    // Banners apply these with /15-/30 alpha modifiers, so we need solid
    // base colors that survive the alpha pass.
    "kumo-info":          v("oklch(70% 0.10 240)"),       // muted blue
    "kumo-info-tint":     v("oklch(70% 0.10 240)"),
    "kumo-success":       v("oklch(70% 0.13 150)"),       // muted green
    "kumo-success-tint":  v("oklch(70% 0.13 150)"),
    "kumo-warning":       v("oklch(80% 0.16 80)"),        // amber
    "kumo-warning-tint":  v("oklch(80% 0.16 80)"),
    "kumo-danger":        v(accent),                      // editorial red
    "kumo-danger-tint":   v(accent),
  },
  typography: {},
};
