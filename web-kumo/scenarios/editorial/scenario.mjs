/**
 * Editorial demo scenario — Acme Corp multitenancy showcase.
 *
 * One file owns the entire scenario: theme tokens, DevAccounts panel
 * entries, and the backend seed script. Vite auto-discovers this file
 * via `import.meta.glob("/scenarios/* /scenario.mjs")`; Node scripts
 * discover it via `fs.readdirSync("scenarios")`. There is NO registry
 * of scenarios to maintain — drop a new folder, that's the only step.
 *
 * The folder name (`editorial/`) MUST equal `SCENARIO_NAME` so that
 * `<html data-theme="...">` resolves the matching palette CSS.
 *
 * @typedef {import("../../src/seed-scenarios/types").Scenario} Scenario
 */

export const SCENARIO_NAME = "editorial";
export const DESCRIPTION = "Acme Corp — 6 orgs, multi-role bob, pending-invites carol, unicode test";
export const PASSWORD = "demo-password-123";

// ============================================================
// THEME — feeds scripts/theme-generator/. Emitted CSS is scoped to
// [data-theme="editorial"] so it coexists with other themes.
// ============================================================

export const THEME = {
  colorScheme: "dark",
  colorSchemeLight: "light",
  palette: {
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
  },
  fonts: {
    "font-display": '"Doto", "Space Mono", monospace',
    "font-body":    '"Space Grotesk", "DM Sans", system-ui, sans-serif',
    "font-mono":    '"Space Mono", "JetBrains Mono", "SF Mono", monospace',
  },
  scale: {
    "display-xl":  "72px", "display-lg":  "48px", "display-md":  "36px",
    "heading":     "24px", "subheading":  "18px",
    "body":        "16px", "body-sm":     "14px",
    "caption":     "12px", "label":       "11px",
    "space-2xs":   "2px",  "space-xs":    "4px",  "space-sm":    "8px",
    "space-md":    "16px", "space-lg":    "24px", "space-xl":    "32px",
    "space-2xl":   "48px", "space-3xl":   "64px", "space-4xl":   "96px",
    "radius-card":    "16px", "radius-compact": "8px",
    "radius-tech":    "4px",  "radius-pill":    "999px",
    "motion-fast":  "150ms", "motion-base":  "250ms",
    "easing":       "cubic-bezier(0.25, 0.1, 0.25, 1)",
  },
  kumoOverrides: {
    text: {
      "kumo-default":     "var(--text-primary)",
      "kumo-strong":      "var(--text-display)",
      "kumo-subtle":      "var(--text-secondary)",
      "kumo-inactive":    "var(--text-disabled)",
      "kumo-placeholder": "var(--text-disabled)",
      "kumo-brand":       "var(--accent)",
      "kumo-link":        "var(--accent)",
    },
    color: {
      "kumo-canvas":      "var(--surface)",
      "kumo-elevated":    "var(--surface-raised)",
      "kumo-recessed":    "var(--surface)",
      "kumo-base":        "var(--surface-raised)",
      "kumo-tint":        "var(--surface-raised)",
      "kumo-overlay":     "var(--surface-raised)",
      "kumo-control":     "var(--surface-raised)",
      "kumo-fill":        "var(--surface-raised)",
      "kumo-fill-hover":  "var(--border)",
      "kumo-interact":    "var(--border-visible)",
      "kumo-contrast":    "var(--text-display)",
      "kumo-line":        "var(--border)",
      "kumo-hairline":    "var(--border-visible)",
      "kumo-focus":       "var(--accent)",
      "kumo-brand":       "var(--accent)",
      "kumo-brand-hover": "var(--accent)",
      "kumo-shadow-edge": "transparent",
      "kumo-shadow-drop": "transparent",
      "kumo-info":          "oklch(70% 0.10 240)",
      "kumo-info-tint":     "oklch(70% 0.10 240)",
      "kumo-success":       "oklch(70% 0.13 150)",
      "kumo-success-tint":  "oklch(70% 0.13 150)",
      "kumo-warning":       "oklch(80% 0.16 80)",
      "kumo-warning-tint":  "oklch(80% 0.16 80)",
      "kumo-danger":        "var(--accent)",
      "kumo-danger-tint":   "var(--accent)",
    },
  },
};

// ============================================================
// ACCOUNTS — DevAccounts panel sign-in cards.
// ============================================================

export const ACCOUNTS = [
  {
    email: "alice@acme.example",
    label: "Alice (owner of 6 orgs)",
    scenario: "Owns Acme + 5 other orgs; multi-org scope-switcher demo",
    landAt: "/",
    badge: "orange",
  },
  {
    email: "bob@acme.example",
    label: "Bob (multi-role)",
    scenario: "Member of Acme/Engineering, owner of Marketing — mixed-role rows",
    landAt: "/",
    badge: "blue",
  },
  {
    email: "carol@partner.example",
    label: "Carol (5 pending invites)",
    scenario: "Lots of pending invites across different orgs — invitations volume",
    landAt: "/invitations",
    badge: "purple",
  },
  {
    email: "dave@late.example",
    label: "Dave (pending billing invite)",
    scenario: "One pending invite to alice's billing scope",
    landAt: "/invitations",
    badge: "teal",
  },
];

// ============================================================
// SEED — backend RPC sequence that materializes the scenario.
// Helpers (rpc/login/ensureUser/ensureOrg/etc.) come from
// scripts/seed/helpers.mjs and are bound to the active base URL.
// ============================================================

export async function seed(h) {
  console.log("[seed:editorial] core users");
  const alice = await h.ensureUser("alice@acme.example");
  const carol = await h.ensureUser("carol@partner.example");
  const dave  = await h.ensureUser("dave@late.example");

  console.log("[seed:editorial] alice's orgs (6)");
  const acme = await h.ensureOrg(alice, "Acme");
  const eng  = await h.ensureOrg(alice, "Engineering");
  const mkt  = await h.ensureOrg(alice, "Marketing");
  const sales = await h.ensureOrg(alice, "Sales");
  const ops  = await h.ensureOrg(alice, "Operations");
  const bigName = await h.ensureOrg(
    alice,
    "Org Name That Really Stretches Mobile Tables to Their Limit"
  );

  console.log("[seed:editorial] bob — multi-role");
  let bob = await h.login("bob@acme.example");
  if (!bob) {
    const inv = await h.tryInviteToOrg(alice, acme.id, "bob@acme.example");
    bob = inv
      ? await h.ensureUserWithInvite("bob@acme.example", inv.token)
      : await h.ensureUser("bob@acme.example");
  }
  await h.tryInviteToOrg(alice, eng.id, "bob@acme.example");
  await h.tryInviteToOrg(alice, mkt.id, "bob@acme.example", "ROLE_OWNER");

  console.log("[seed:editorial] org members (auto-accept on signup)");
  await h.inviteAndJoin(alice, eng.id, "eve@design.example");
  await h.inviteAndJoin(alice, eng.id, "frank@engineering.example");
  await h.inviteAndJoin(alice, mkt.id, "grace@marketing.example");
  await h.inviteAndJoin(alice, sales.id, "henry@sales.example");
  await h.inviteAndJoin(alice, sales.id, "ivy@a-rather-long-domain-name.example");
  await h.inviteAndJoin(alice, ops.id, "jake@ops.example");
  await h.inviteAndJoin(alice, ops.id, "kate@ops.example");
  await h.inviteAndJoin(alice, bigName.id, "liam@team.example");
  await h.inviteAndJoin(alice, acme.id, "über@müller.example");

  console.log("[seed:editorial] carol — pending invites galore");
  await h.tryInviteToOrg(alice, acme.id, "carol@partner.example");
  await h.tryInviteToOrg(alice, eng.id, "carol@partner.example");
  await h.tryInviteToOrg(alice, sales.id, "carol@partner.example");
  await h.tryInviteToOrg(alice, ops.id, "carol@partner.example");
  await h.tryInviteToOrg(alice, bigName.id, "carol@partner.example", "ROLE_OWNER");

  console.log("[seed:editorial] dave — pending billing invite");
  await h.tryInviteToBilling(alice, alice.whoami.billingAccountId, "dave@late.example");

  return {
    users: {
      alice: { email: alice.email, token: alice.token, whoami: alice.whoami },
      bob:   { email: bob.email,   token: bob.token,   whoami: bob.whoami },
      carol: { email: carol.email, token: carol.token, whoami: carol.whoami },
      dave:  { email: dave.email,  token: dave.token,  whoami: dave.whoami },
    },
    orgs: {
      acme: acme.id, engineering: eng.id, marketing: mkt.id,
      sales: sales.id, operations: ops.id, longName: bigName.id,
    },
    notes: {
      alice: "/billing → 6 orgs visible; long-name overflow + scope-switcher density",
      bob:   "/ → multi-role: member of Acme/Eng, owner of Marketing",
      carol: "/invitations → 5 pending across different orgs",
      dave:  "/invitations → 1 pending billing invite",
    },
  };
}
