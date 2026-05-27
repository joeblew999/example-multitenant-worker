#!/usr/bin/env node
/**
 * Seed the local D1 with realistic data so the GUI has volume to render.
 *
 * ## Naming convention: all seed emails use the `.example` TLD
 *
 * Per RFC 6761 the `.example` TLD is permanently reserved for
 * documentation and never resolves on the public internet. So a real
 * customer signing up with their real email can NEVER collide with a
 * seed user, and the seed is safe to run against production. Devs can
 * log into prod as alice@acme.example to test the live system without
 * any risk of shadowing a real account.
 *
 * Keep this convention when adding new seed users — anything ending in
 * `.example` is fair game, anything else risks colliding with a real
 * customer email.
 *
 * Scenario:
 *   - alice owns 6 orgs under her billing account (Acme, Engineering,
 *     Marketing, Sales, Operations + a deliberately long-named org)
 *   - bob is a multi-role player: owner of one org, member of others
 *   - carol is the "lots of pending invites" demo — 5 pending across
 *     different orgs, different inviters
 *   - 8+ minor users populate org member lists
 *   - one unicode user (über@müller.example) tests font rendering
 *
 * Outputs a manifest at .seed.json with sessionTokens for each user.
 *
 * Usage:
 *   mise run seed:dev               (local — :8787 wrangler)
 *   mise run seed:prod              (deployed — reads URL from fnox)
 *   BASE=https://... node scripts/seed.mjs
 *
 * Idempotent — re-running is safe; users/orgs are detected and reused.
 */

import { writeFile } from "node:fs/promises";

process.env.NODE_TLS_REJECT_UNAUTHORIZED = "0";
process.removeAllListeners("warning");

const BASE = process.env.BASE ?? "https://localhost:8787";
const PASSWORD = "demo-password-123";

async function rpc(service, method, body, token) {
  const url = `${BASE}/workers.${service}/${method}`;
  const res = await fetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    body: JSON.stringify(body ?? {}),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`${service}/${method} → ${res.status}: ${text}`);
  }
  return res.json();
}

async function login(email) {
  try {
    const r = await rpc("auth.v1.AuthService", "Login", { email, password: PASSWORD });
    return { email, token: r.sessionToken, whoami: r.whoami };
  } catch {
    return null;
  }
}

async function ensureUser(email) {
  const existing = await login(email);
  if (existing) return existing;
  const r = await rpc("auth.v1.AuthService", "Signup", { email, password: PASSWORD });
  return { email, token: r.sessionToken, whoami: r.whoami };
}

async function ensureUserWithInvite(email, inviteToken) {
  const existing = await login(email);
  if (existing) return existing;
  const r = await rpc("auth.v1.AuthService", "Signup", {
    email, password: PASSWORD, inviteToken,
  });
  return { email, token: r.sessionToken, whoami: r.whoami };
}

async function ensureOrg(user, displayName) {
  const billingAccountId = user.whoami.billingAccountId;
  try {
    const list = await rpc("org.v1.OrgService", "ListOrganizations",
      { billingAccountId }, user.token);
    const hit = (list.organizations || []).find((o) => o.displayName === displayName && !o.personal);
    if (hit) return hit;
  } catch {}
  const r = await rpc("org.v1.OrgService", "CreateOrganization",
    { billingAccountId, displayName }, user.token);
  return r.organization;
}

async function tryInviteToOrg(user, orgId, email, role = "ROLE_MEMBER") {
  try {
    const r = await rpc("org.v1.OrgService", "InviteMember",
      { orgId, email, role }, user.token);
    return { invitationId: r.invitationId, token: r.inviteTokenForDemo };
  } catch {
    return null;
  }
}

async function tryInviteToBilling(user, billingAccountId, email, role = "ROLE_MEMBER") {
  try {
    const r = await rpc("billing.v1.BillingService", "InviteMember",
      { billingAccountId, email, role }, user.token);
    return { invitationId: r.invitationId, token: r.inviteTokenForDemo };
  } catch {
    return null;
  }
}

/** Invite + auto-accept by signup — for batch member-population. */
async function inviteAndJoin(inviter, orgId, email) {
  const existing = await login(email);
  if (existing) return existing; // already exists, can't re-invite
  const inv = await tryInviteToOrg(inviter, orgId, email);
  if (!inv) return await ensureUser(email);
  return await ensureUserWithInvite(email, inv.token);
}

async function main() {
  console.log(`[seed] base = ${BASE}`);
  const health = await fetch(`${BASE}/healthz`);
  if (!health.ok) throw new Error(`worker not reachable at ${BASE}/healthz`);

  // ──────────────────────────────────────────────────────────
  // Core users
  // ──────────────────────────────────────────────────────────
  console.log("[seed] core users");
  const alice = await ensureUser("alice@acme.example");
  const carol = await ensureUser("carol@partner.example");
  const dave  = await ensureUser("dave@late.example");

  // ──────────────────────────────────────────────────────────
  // Alice's portfolio of orgs
  // ──────────────────────────────────────────────────────────
  console.log("[seed] alice's orgs (6)");
  const acme = await ensureOrg(alice, "Acme");
  const eng  = await ensureOrg(alice, "Engineering");
  const mkt  = await ensureOrg(alice, "Marketing");
  const sales = await ensureOrg(alice, "Sales");
  const ops  = await ensureOrg(alice, "Operations");
  const bigName = await ensureOrg(
    alice,
    "Org Name That Really Stretches Mobile Tables to Their Limit"
  );

  // ──────────────────────────────────────────────────────────
  // Bob — multi-role: member of Acme + Eng, owner of Marketing
  // ──────────────────────────────────────────────────────────
  console.log("[seed] bob — multi-role");
  let bob = await login("bob@acme.example");
  if (!bob) {
    const inv = await tryInviteToOrg(alice, acme.id, "bob@acme.example");
    bob = inv
      ? await ensureUserWithInvite("bob@acme.example", inv.token)
      : await ensureUser("bob@acme.example");
  }
  await tryInviteToOrg(alice, eng.id, "bob@acme.example");
  await tryInviteToOrg(alice, mkt.id, "bob@acme.example", "ROLE_OWNER");

  // ──────────────────────────────────────────────────────────
  // Other org members (accept invites on signup) — all `.example` TLD
  // ──────────────────────────────────────────────────────────
  console.log("[seed] org members (auto-accept on signup)");
  await inviteAndJoin(alice, eng.id, "eve@design.example");
  await inviteAndJoin(alice, eng.id, "frank@engineering.example");
  await inviteAndJoin(alice, mkt.id, "grace@marketing.example");
  await inviteAndJoin(alice, sales.id, "henry@sales.example");
  await inviteAndJoin(alice, sales.id, "ivy@a-rather-long-domain-name.example");
  await inviteAndJoin(alice, ops.id, "jake@ops.example");
  await inviteAndJoin(alice, ops.id, "kate@ops.example");
  await inviteAndJoin(alice, bigName.id, "liam@team.example");

  // Unicode display test (non-ASCII local part + non-ASCII domain).
  await inviteAndJoin(alice, acme.id, "über@müller.example");

  // ──────────────────────────────────────────────────────────
  // Carol — the "lots of pending invites" demo user
  // ──────────────────────────────────────────────────────────
  console.log("[seed] carol — pending invites galore");
  await tryInviteToOrg(alice, acme.id, "carol@partner.example");
  await tryInviteToOrg(alice, eng.id, "carol@partner.example");
  await tryInviteToOrg(alice, sales.id, "carol@partner.example");
  await tryInviteToOrg(alice, ops.id, "carol@partner.example");
  await tryInviteToOrg(alice, bigName.id, "carol@partner.example", "ROLE_OWNER");

  // ──────────────────────────────────────────────────────────
  // Dave — pending billing invite (left over from v1 demo)
  // ──────────────────────────────────────────────────────────
  console.log("[seed] dave — pending billing invite");
  await tryInviteToBilling(alice, alice.whoami.billingAccountId, "dave@late.example", "ROLE_MEMBER");

  // ──────────────────────────────────────────────────────────
  // Manifest
  // ──────────────────────────────────────────────────────────
  const manifest = {
    base: BASE,
    password: PASSWORD,
    users: {
      alice: { email: alice.email, token: alice.token, whoami: alice.whoami },
      bob:   { email: bob.email,   token: bob.token,   whoami: bob.whoami },
      carol: { email: carol.email, token: carol.token, whoami: carol.whoami },
      dave:  { email: dave.email,  token: dave.token,  whoami: dave.whoami },
    },
    orgs: { acme: acme.id, engineering: eng.id, marketing: mkt.id, sales: sales.id, operations: ops.id, longName: bigName.id },
    notes: {
      "tour-as": {
        alice: "/billing → 6 orgs visible; sees long-name overflow + scope-switcher density",
        bob:   "/ → multi-role: member of Acme/Eng, owner of Marketing",
        carol: "/invitations → 5 pending across different orgs (stresses Invitations table)",
        dave:  "/invitations → 1 pending billing invite",
      },
    },
  };
  await writeFile(".seed.json", JSON.stringify(manifest, null, 2));

  console.log("\n[seed] done. .seed.json written.");
  console.log(`Password: ${PASSWORD}`);
  console.log("Tour the volume scenarios:");
  for (const [u, note] of Object.entries(manifest.notes["tour-as"])) {
    console.log(`  ${u}@…  →  ${note}`);
  }
}

main().catch((err) => {
  console.error(`[seed] FAILED: ${err.message}`);
  process.exit(1);
});
