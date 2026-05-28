/**
 * RemySports / ChampsCircuit demo scenario — Thai basketball platform
 * admin-GUI view.
 *
 * Source data: joeblew999/remy-sport-biz/data/seed/*.csv (the product
 * owner's fictional-but-realistic pilot dataset for Thailand basketball
 * tournaments — Thai schools, federations, coaches, players).
 *
 * The remy-sport-biz repo defines a deep basketball domain (events,
 * matches, brackets, venues, notifications, player positions, etc.).
 * THIS demo only models the slice that fits the connectrpc-cedar
 * admin-GUI scaffold: identity (users + roles) and tenancy (orgs +
 * memberships + invitations). Match/event/venue data lives in a
 * separate ChampsCircuit Worker that builds ON TOP of this auth+admin
 * layer — see SEED.md §10.
 *
 * Mapping (ChampsCircuit → multitenant-worker schema):
 *   role_code (ADMIN/ORGANIZER/COACH/PLAYER/...) → org_membership.role
 *     (HEAD coach + ORGANIZER + ADMIN → OWNER; assistant + player → MEMBER)
 *   org (school / club / federation) → organization (under admin's billing)
 *   team                              → organization (sibling of school)
 *   team_coaches.HEAD                 → org_membership OWNER
 *   team_coaches.ASSISTANT            → org_membership MEMBER
 *   player_teams                      → org_membership MEMBER (only for
 *                                       players with linked user_id —
 *                                       minors-without-account is a
 *                                       ChampsCircuit nuance our schema
 *                                       doesn't model)
 *   PENDING_APPROVAL referee          → pending invitation (no user yet)
 *
 * Display names are bilingual "ภาษาไทย (English)" — Noto Sans Thai
 * is in the remysport theme's body font stack, so Thai script renders
 * correctly. This forces the demo to look like a real Thai SaaS, not
 * a Western app with Thai labels bolted on.
 *
 * @typedef {import("../../src/seed-scenarios/types").Scenario} Scenario
 */

export const SCENARIO_NAME = "remysport";
export const DESCRIPTION = "ChampsCircuit — Thai basketball platform: 9 orgs (4 schools + federation + 4 teams), 6 actor types, bilingual display names";
export const PASSWORD = "demo-password-123";

// ============================================================
// THEME — unchanged from prior remysport (paper bg, marigold orange,
// Inter / Space Grotesk / IBM Plex Mono, Noto Sans Thai in body stack).
// ============================================================

export const THEME = {
  colorScheme: "light",
  palette: {
    base: {
      "paper":           "oklch(0.985 0.005 90)",
      "paper-2":         "oklch(0.965 0.008 90)",
      "paper-3":         "oklch(0.93 0.01 80)",
      "rule":            "oklch(0.85 0.005 90)",
      "black":           "oklch(0.985 0.005 90)",
      "surface":         "oklch(0.985 0.005 90)",
      "surface-raised":  "oklch(0.965 0.008 90)",
      "border":          "oklch(0.85 0.005 90)",
      "border-visible":  "oklch(0.78 0.01 80)",
      "text-disabled":   "oklch(0.65 0.005 270)",
      "text-secondary":  "oklch(0.55 0.01 270)",
      "text-primary":    "oklch(0.32 0.01 270)",
      "text-display":    "oklch(0.18 0.01 270)",
      "accent":          "oklch(0.62 0.18 35)",
      "accent-subtle":   "oklch(0.62 0.18 35 / 0.15)",
      "success":         "oklch(0.55 0.14 145)",
      "warning":         "oklch(0.75 0.16 75)",
      "error":           "oklch(0.55 0.22 25)",
      "interactive":     "oklch(0.48 0.18 35)",
    },
    light: {},
  },
  fonts: {
    "font-display": '"Space Grotesk", "DM Sans", system-ui, sans-serif',
    "font-body":    '"Inter", "Noto Sans Thai", system-ui, sans-serif',
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
// ACCOUNTS — 6 DevAccount cards, one per actor type that exists in
// the ChampsCircuit data model (Admin / Organizer / Coach / Player /
// Spectator / Referee). Sourced from remy-sport-biz/data/seed/users.csv.
//
// Each email matches a user the seed() function below actually creates.
// The translation rule: CSV's `.test` TLD → our convention's `.example`
// TLD (both RFC-reserved; we use .example per SEED.md §3).
// ============================================================

export const ACCOUNTS = [
  {
    email: "admin@remysport.example",
    label: "System Admin — Platform overseer",
    scenario: "Owns the platform billing — every org/team is under their tenancy. Sees everything.",
    landAt: "/",
    badge: "orange",
  },
  {
    email: "somchai.p@assumption.example",
    label: "สมชาย พัฒนสกุล — Organizer (Assumption)",
    scenario: "Owns Assumption College school + its U16/U18 boys teams. Runs tournaments for the school.",
    landAt: "/",
    badge: "blue",
  },
  {
    email: "wichai.s@assumption.example",
    label: "วิชัย ศรีสุข — Head Coach (Assumption)",
    scenario: "OWNER of Assumption U16 Boys + U18 Boys. Manages two team rosters across the season.",
    landAt: "/",
    badge: "purple",
  },
  {
    email: "thanakorn.s@personal.example",
    label: "ธนกร สุขใส — Player (transferred mid-season)",
    scenario: "Member of Assumption U16 Boys AND U18 Boys — example of mid-season promotion across rosters.",
    landAt: "/",
    badge: "teal",
  },
  {
    email: "pim.s@personal.example",
    label: "พิม สุขใส — Spectator (parent of Thanakorn)",
    scenario: "Parent following her son's team. One pending supporter invite to Assumption College.",
    landAt: "/invitations",
    badge: "orange",
  },
  {
    email: "adisorn.b@bat.example",
    label: "อดิศร บุญชัย — Referee (BSBL-accredited)",
    scenario: "Accredited referee — member of the Basketball Sport Association of Thailand federation.",
    landAt: "/",
    badge: "blue",
  },
];

// ============================================================
// SEED — builds the org tree + memberships + invites.
//
// Architecture: admin signs up first; the admin's personal billing
// account becomes the "platform tenancy" — every org/team is created
// under it. Other users sign up via invites and get memberships in
// the orgs that match their CSV-defined roles.
//
// This mirrors a real Thai platform admin's view: one tenancy holding
// the entire ecosystem of schools, federation, and teams.
// ============================================================

export async function seed(h) {
  console.log("[seed:remysport] admin — platform owner");
  const admin = await h.ensureUser("admin@remysport.example");

  console.log("[seed:remysport] orgs — 4 schools/clubs + 1 federation");
  // Bilingual display names: "ภาษาไทย (English)" — matches the CSV.
  const assumption = await h.ensureOrg(admin, "โรงเรียนอัสสัมชัญ (Assumption College)");
  const triamUdom = await h.ensureOrg(admin, "โรงเรียนเตรียมอุดมศึกษา (Triam Udom Suksa School)");
  const montfort  = await h.ensureOrg(admin, "โรงเรียนมงฟอร์ตวิทยาลัย (Montfort College)");
  const bsbl      = await h.ensureOrg(admin, "สมาคมกีฬาบาสเกตบอลแห่งประเทศไทย (Basketball Sport Association of Thailand)");
  const bangkokBc = await h.ensureOrg(admin, "สโมสรบาสเกตบอลกรุงเทพ (Bangkok Basketball Club)");

  console.log("[seed:remysport] teams — 4 from teams.csv");
  const teamAsmU16Boys = await h.ensureOrg(admin, "ทีมบาสเกตบอลอัสสัมชัญ U16 ชาย (Assumption U16 Boys)");
  const teamAsmU18Boys = await h.ensureOrg(admin, "ทีมบาสเกตบอลอัสสัมชัญ U18 ชาย (Assumption U18 Boys)");
  const teamTuU18Girls = await h.ensureOrg(admin, "ทีมบาสเกตบอลเตรียมอุดมศึกษา U18 หญิง (Triam Udom U18 Girls)");
  const teamMfU16Boys  = await h.ensureOrg(admin, "ทีมบาสเกตบอลมงฟอร์ต U16 ชาย (Montfort U16 Boys)");

  // ──────────────────────────────────────────────────────────
  // Organizers (3 from users.csv)
  // Each organizer is invited as OWNER of the school they run.
  // ──────────────────────────────────────────────────────────
  console.log("[seed:remysport] organizers");
  await h.inviteAndJoin(admin, assumption.id, "somchai.p@assumption.example", "ROLE_OWNER"); // Somchai @ Assumption
  await h.inviteAndJoin(admin, bsbl.id, "niran.w@bsbl.example", "ROLE_OWNER");               // Niran @ BSBL
  await h.inviteAndJoin(admin, montfort.id, "apinya.k@cmcamp.example", "ROLE_OWNER");        // Apinya @ Montfort

  // ──────────────────────────────────────────────────────────
  // Coaches (3 from team_coaches.csv)
  // HEAD → OWNER of the team; ASSISTANT → MEMBER.
  // ──────────────────────────────────────────────────────────
  console.log("[seed:remysport] coaches");
  // Wichai: HEAD of Assumption U16 Boys + Assumption U18 Boys
  await h.inviteAndJoin(admin, teamAsmU16Boys.id, "wichai.s@assumption.example", "ROLE_OWNER");
  await h.tryInviteToOrg(admin, teamAsmU18Boys.id, "wichai.s@assumption.example", "ROLE_OWNER");

  // Pranom: HEAD of Triam Udom U18 Girls + ASSISTANT at Assumption U16 Boys
  await h.inviteAndJoin(admin, teamTuU18Girls.id, "pranom.c@triamudom.example", "ROLE_OWNER");
  await h.tryInviteToOrg(admin, teamAsmU16Boys.id, "pranom.c@triamudom.example", "ROLE_MEMBER");

  // Sutee: HEAD of Montfort U16 Boys
  await h.inviteAndJoin(admin, teamMfU16Boys.id, "sutee.k@montfort.example", "ROLE_OWNER");

  // ──────────────────────────────────────────────────────────
  // Players (only the 2 with user accounts in players.csv)
  // ──────────────────────────────────────────────────────────
  console.log("[seed:remysport] players");
  // Thanakorn: on Assumption U16 Boys AND U18 Boys (mid-season transfer per CSV)
  let thanakorn = await h.login("thanakorn.s@personal.example");
  if (!thanakorn) {
    const inv = await h.tryInviteToOrg(admin, teamAsmU16Boys.id, "thanakorn.s@personal.example");
    thanakorn = inv
      ? await h.ensureUserWithInvite("thanakorn.s@personal.example", inv.token)
      : await h.ensureUser("thanakorn.s@personal.example");
  }
  await h.tryInviteToOrg(admin, teamAsmU18Boys.id, "thanakorn.s@personal.example");

  // Kanya: on Triam Udom U18 Girls
  await h.inviteAndJoin(admin, teamTuU18Girls.id, "kanya.t@personal.example");

  // ──────────────────────────────────────────────────────────
  // Referee (1 active, 1 pending — matches users.csv statuses)
  // Active referees are members of the federation; pending = invite only.
  // ──────────────────────────────────────────────────────────
  console.log("[seed:remysport] referees");
  await h.inviteAndJoin(admin, bsbl.id, "adisorn.b@bat.example");

  // ──────────────────────────────────────────────────────────
  // Spectator / Parent (1 from users.csv — Pim is Thanakorn's mom)
  // Sign up the user, then create one pending invite as a "supporter"
  // to Assumption (the school of her son's team).
  // ──────────────────────────────────────────────────────────
  console.log("[seed:remysport] spectator / parent");
  const pim = await h.ensureUser("pim.s@personal.example");
  await h.tryInviteToOrg(admin, assumption.id, "pim.s@personal.example");

  // ──────────────────────────────────────────────────────────
  // Pending invitations to non-yet-existing users
  // (waraporn = PENDING_APPROVAL referee, no account until accreditation)
  // ──────────────────────────────────────────────────────────
  console.log("[seed:remysport] pending invites");
  await h.tryInviteToOrg(admin, bsbl.id, "waraporn.j@bat.example", "ROLE_MEMBER");

  // ──────────────────────────────────────────────────────────
  // Manifest
  // ──────────────────────────────────────────────────────────
  // Re-fetch principal users to get fresh whoami payloads.
  const somchai  = await h.login("somchai.p@assumption.example");
  const wichai   = await h.login("wichai.s@assumption.example");
  const adisorn  = await h.login("adisorn.b@bat.example");

  return {
    users: {
      admin:     { email: admin.email,     token: admin.token,     whoami: admin.whoami },
      somchai:   { email: somchai.email,   token: somchai.token,   whoami: somchai.whoami },
      wichai:    { email: wichai.email,    token: wichai.token,    whoami: wichai.whoami },
      thanakorn: { email: thanakorn.email, token: thanakorn.token, whoami: thanakorn.whoami },
      pim:       { email: pim.email,       token: pim.token,       whoami: pim.whoami },
      adisorn:   { email: adisorn.email,   token: adisorn.token,   whoami: adisorn.whoami },
    },
    orgs: {
      assumption: assumption.id, triamUdom: triamUdom.id, montfort: montfort.id,
      bsbl: bsbl.id, bangkokBc: bangkokBc.id,
      teamAsmU16Boys: teamAsmU16Boys.id, teamAsmU18Boys: teamAsmU18Boys.id,
      teamTuU18Girls: teamTuU18Girls.id, teamMfU16Boys: teamMfU16Boys.id,
    },
    notes: {
      admin:     "/ → owns the platform billing; sees all 9 orgs",
      somchai:   "/ → organizer at Assumption College; runs the school's tournaments",
      wichai:    "/ → head coach of Assumption U16 + U18 Boys (multi-team OWNER)",
      thanakorn: "/ → player on Assumption U16 Boys + U18 Boys (mid-season transfer)",
      pim:       "/invitations → pending supporter invite to Assumption (her son's school)",
      adisorn:   "/ → accredited referee at the Basketball Sport Association of Thailand",
    },
  };
}
