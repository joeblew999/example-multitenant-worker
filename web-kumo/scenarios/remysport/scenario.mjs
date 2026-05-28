/**
 * RemySports demo scenario — Bangkok Suns basketball club tenancy.
 *
 * One file owns the entire scenario: theme, DevAccounts, backend seed.
 * See scenarios/editorial/scenario.mjs for the architecture rationale.
 *
 * Domain mapping (data model → basketball):
 *   billing account  ←→  club / franchise (pays for the platform)
 *   organization     ←→  team within the club
 *   role             ←→  coach (OWNER) / player or staff (MEMBER)
 *
 * Branding from joeblew999/remy-sport-biz/pitchdeck/index.html: paper-bg
 * light theme, cool dark blue-gray ink, warm Thai-marigold orange accent,
 * Inter body + Space Grotesk display + IBM Plex Mono.
 *
 * @typedef {import("../../src/seed-scenarios/types").Scenario} Scenario
 */

export const SCENARIO_NAME = "remysport";
export const DESCRIPTION = "Bangkok Suns basketball club — 5 teams, multi-team captain, scout invites";
export const PASSWORD = "demo-password-123";

// ============================================================
// THEME
// ============================================================

export const THEME = {
  colorScheme: "light",
  palette: {
    base: {
      // Surfaces — warm paper tones
      "paper":           "oklch(0.985 0.005 90)",
      "paper-2":         "oklch(0.965 0.008 90)",
      "paper-3":         "oklch(0.93 0.01 80)",
      "rule":            "oklch(0.85 0.005 90)",
      // Editorial-shaped vars so the chrome (.shell/.topbar/.brand/.page)
      // renders correctly under remysport without per-chrome forks.
      "black":           "oklch(0.985 0.005 90)",
      "surface":         "oklch(0.985 0.005 90)",
      "surface-raised":  "oklch(0.965 0.008 90)",
      "border":          "oklch(0.85 0.005 90)",
      "border-visible":  "oklch(0.78 0.01 80)",
      // Ink — cool dark blue-gray
      "text-disabled":   "oklch(0.65 0.005 270)",
      "text-secondary":  "oklch(0.55 0.01 270)",
      "text-primary":    "oklch(0.32 0.01 270)",
      "text-display":    "oklch(0.18 0.01 270)",
      // Accent — warm Thai marigold
      "accent":          "oklch(0.62 0.18 35)",
      "accent-subtle":   "oklch(0.62 0.18 35 / 0.15)",
      "success":         "oklch(0.55 0.14 145)",
      "warning":         "oklch(0.75 0.16 75)",
      "error":           "oklch(0.55 0.22 25)",
      "interactive":     "oklch(0.48 0.18 35)",
    },
    light: {},   // base IS light; no prefers-color-scheme variant
  },
  fonts: {
    "font-display": '"Space Grotesk", "DM Sans", system-ui, sans-serif',
    "font-body":    '"Inter", system-ui, sans-serif',
    "font-mono":    '"IBM Plex Mono", "JetBrains Mono", "SF Mono", monospace',
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
      "kumo-brand-hover": "var(--interactive)",
      "kumo-shadow-edge": "transparent",
      "kumo-shadow-drop": "transparent",
      "kumo-info":          "oklch(70% 0.10 240)",
      "kumo-info-tint":     "oklch(70% 0.10 240)",
      "kumo-success":       "var(--success)",
      "kumo-success-tint":  "var(--success)",
      "kumo-warning":       "var(--warning)",
      "kumo-warning-tint":  "var(--warning)",
      "kumo-danger":        "var(--error)",
      "kumo-danger-tint":   "var(--error)",
    },
  },
};

// ============================================================
// ACCOUNTS
// ============================================================

export const ACCOUNTS = [
  {
    email: "coach@bangkok-suns.example",
    label: "Coach (owns 5 teams)",
    scenario: "Owns Bangkok Suns + 5 teams; multi-team scope-switcher demo",
    landAt: "/",
    badge: "orange",
  },
  {
    email: "captain@suns.example",
    label: "Captain (multi-team)",
    scenario: "Member of Senior A, owner of Senior B — mixed-role rows",
    landAt: "/",
    badge: "blue",
  },
  {
    email: "scout@asia-league.example",
    label: "Scout (5 pending trials)",
    scenario: "Pending team-trial invites across different squads",
    landAt: "/invitations",
    badge: "purple",
  },
  {
    email: "manager@suns-academy.example",
    label: "Manager (pending club-staff invite)",
    scenario: "One pending invite to join the club's billing scope as staff",
    landAt: "/invitations",
    badge: "teal",
  },
];

// ============================================================
// SEED
// ============================================================

export async function seed(h) {
  console.log("[seed:remysport] core users");
  const coach   = await h.ensureUser("coach@bangkok-suns.example");
  const scout   = await h.ensureUser("scout@asia-league.example");
  const manager = await h.ensureUser("manager@suns-academy.example");

  console.log("[seed:remysport] coach's teams (5 + long-name)");
  const club    = await h.ensureOrg(coach, "Bangkok Suns");
  const seniorA = await h.ensureOrg(coach, "Senior A");
  const seniorB = await h.ensureOrg(coach, "Senior B");
  const u18Boys = await h.ensureOrg(coach, "U18 Boys");
  const u16Girls = await h.ensureOrg(coach, "U16 Girls");
  const trainingSquad = await h.ensureOrg(
    coach,
    "Bangkok Suns Pre-Season Trial Squad (Closed Sessions, Coaches Only)"
  );

  console.log("[seed:remysport] captain — multi-team");
  let captain = await h.login("captain@suns.example");
  if (!captain) {
    const inv = await h.tryInviteToOrg(coach, club.id, "captain@suns.example");
    captain = inv
      ? await h.ensureUserWithInvite("captain@suns.example", inv.token)
      : await h.ensureUser("captain@suns.example");
  }
  await h.tryInviteToOrg(coach, seniorA.id, "captain@suns.example");
  await h.tryInviteToOrg(coach, seniorB.id, "captain@suns.example", "ROLE_OWNER");

  console.log("[seed:remysport] roster (auto-accept on signup)");
  await h.inviteAndJoin(coach, seniorA.id, "guard@suns.example");
  await h.inviteAndJoin(coach, seniorA.id, "forward@suns.example");
  await h.inviteAndJoin(coach, seniorB.id, "rookie@suns.example");
  await h.inviteAndJoin(coach, u18Boys.id, "wing@u18-boys.example");
  await h.inviteAndJoin(coach, u18Boys.id, "point@u18-boys.example");
  await h.inviteAndJoin(coach, u16Girls.id, "captain@u16-girls.example");
  await h.inviteAndJoin(coach, u16Girls.id, "shooter@u16-girls.example");
  await h.inviteAndJoin(coach, trainingSquad.id, "tryout-04@academy.example");
  await h.inviteAndJoin(coach, club.id, "นักบาส@สโมสร.example");

  console.log("[seed:remysport] scout — pending team-trial invites");
  await h.tryInviteToOrg(coach, club.id, "scout@asia-league.example");
  await h.tryInviteToOrg(coach, seniorA.id, "scout@asia-league.example");
  await h.tryInviteToOrg(coach, u18Boys.id, "scout@asia-league.example");
  await h.tryInviteToOrg(coach, u16Girls.id, "scout@asia-league.example");
  await h.tryInviteToOrg(coach, trainingSquad.id, "scout@asia-league.example", "ROLE_OWNER");

  console.log("[seed:remysport] manager — pending club-staff invite");
  await h.tryInviteToBilling(coach, coach.whoami.billingAccountId, "manager@suns-academy.example");

  return {
    users: {
      coach:   { email: coach.email,   token: coach.token,   whoami: coach.whoami },
      captain: { email: captain.email, token: captain.token, whoami: captain.whoami },
      scout:   { email: scout.email,   token: scout.token,   whoami: scout.whoami },
      manager: { email: manager.email, token: manager.token, whoami: manager.whoami },
    },
    orgs: {
      club: club.id, seniorA: seniorA.id, seniorB: seniorB.id,
      u18Boys: u18Boys.id, u16Girls: u16Girls.id, trainingSquad: trainingSquad.id,
    },
    notes: {
      coach:   "/billing → 5 teams + trial squad; club is billing root",
      captain: "/ → multi-team: member of Senior A, owner of Senior B",
      scout:   "/invitations → 5 pending trials across teams",
      manager: "/invitations → 1 pending club-staff invite",
    },
  };
}
