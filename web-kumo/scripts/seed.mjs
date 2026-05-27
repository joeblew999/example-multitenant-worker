#!/usr/bin/env node
/**
 * Seed the local D1 with realistic data so the GUI has something to
 * render. Uses Connect RPC JSON over fetch — no SDK required.
 *
 * Creates:
 *   - alice@acme.io      (owner of an "Acme" org, owner of personal billing)
 *   - bob@acme.io        (member of Acme via accepted invite)
 *   - carol@partner.dev  (pending invite to Acme — never accepted)
 *   - dave@late.io       (pending invite — for "stale" demo)
 *
 * Outputs a manifest at .seed.json with sessionTokens for each user so
 * other scripts (or you) can log in as them via DevTools:
 *   localStorage.setItem('wm.session', JSON.stringify({
 *     token: ..., whoami: ...
 *   }))
 *
 * Usage:
 *   mise run seed:dev               (default — :8787 wrangler)
 *   BASE=http://localhost:5175 node scripts/seed.mjs   (proxied via Vite)
 *
 * Reset between runs by restarting the worker (in-memory D1 dev sim) or
 * `wrangler d1 execute DB --local --command "DELETE FROM users"`.
 */

import { writeFile } from "node:fs/promises";

// Worker dev uses self-signed certs (wrangler --local-protocol=https).
// Local-only script — fine to accept any cert.
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
  if (existing) {
    console.log(`  ${email} — already exists, logged in`);
    return existing;
  }
  console.log(`  signup ${email}`);
  const r = await rpc("auth.v1.AuthService", "Signup", { email, password: PASSWORD });
  return { email, token: r.sessionToken, whoami: r.whoami };
}

async function ensureUserWithInvite(email, inviteToken) {
  const existing = await login(email);
  if (existing) {
    console.log(`  ${email} — already exists, logged in (invite not consumed: already a member of relevant scopes)`);
    return existing;
  }
  console.log(`  signup ${email} (accepting invite)`);
  const r = await rpc("auth.v1.AuthService", "Signup", {
    email, password: PASSWORD, inviteToken,
  });
  return { email, token: r.sessionToken, whoami: r.whoami };
}

async function ensureOrg(user, displayName) {
  const billingAccountId = user.whoami.billingAccountId;
  // List orgs already under this billing scope; reuse if one already
  // matches by name (idempotency).
  try {
    const list = await rpc("org.v1.OrgService", "ListOrganizations",
      { billingAccountId }, user.token);
    const hit = (list.organizations || []).find((o) => o.displayName === displayName && !o.personal);
    if (hit) {
      console.log(`  org "${displayName}" — already exists`);
      return hit;
    }
  } catch {}
  console.log(`  create org "${displayName}" under billing ${billingAccountId.slice(0,12)}…`);
  const r = await rpc("org.v1.OrgService", "CreateOrganization",
    { billingAccountId, displayName }, user.token);
  return r.organization;
}

async function tryInviteToOrg(user, orgId, email, role = "ROLE_MEMBER") {
  try {
    const r = await rpc("org.v1.OrgService", "InviteMember",
      { orgId, email, role }, user.token);
    console.log(`  invite ${email} to org ${orgId.slice(0,12)}… as ${role}`);
    return { invitationId: r.invitationId, token: r.inviteTokenForDemo };
  } catch (e) {
    console.log(`  invite ${email} to org — skipped (${e.message.slice(0, 60)})`);
    return null;
  }
}

async function tryInviteToBilling(user, billingAccountId, email, role = "ROLE_MEMBER") {
  try {
    const r = await rpc("billing.v1.BillingService", "InviteMember",
      { billingAccountId, email, role }, user.token);
    console.log(`  invite ${email} to billing ${billingAccountId.slice(0,12)}… as ${role}`);
    return { invitationId: r.invitationId, token: r.inviteTokenForDemo };
  } catch (e) {
    console.log(`  invite ${email} to billing — skipped (${e.message.slice(0, 60)})`);
    return null;
  }
}

async function main() {
  console.log(`[seed] base = ${BASE}`);

  // Health check first so a clean error message beats a stack trace.
  const health = await fetch(`${BASE}/healthz`);
  if (!health.ok) throw new Error(`worker not reachable at ${BASE}/healthz`);

  console.log("[seed] ensuring users");
  const alice = await ensureUser("alice@acme.io");
  const carol = await ensureUser("carol@partner.dev");
  const dave = await ensureUser("dave@late.io");

  console.log("[seed] ensuring Acme org");
  const acme = await ensureOrg(alice, "Acme");

  console.log("[seed] bob — invite + signup");
  // On a fresh DB bob doesn't exist yet, so we need to invite-then-signup
  // with the demo token. On re-run bob already exists, just log him in.
  let bob = await login("bob@acme.io");
  if (!bob) {
    const bobInvite = await tryInviteToOrg(alice, acme.id, "bob@acme.io");
    if (bobInvite) {
      bob = await ensureUserWithInvite("bob@acme.io", bobInvite.token);
    } else {
      bob = await ensureUser("bob@acme.io");
    }
  } else {
    console.log(`  bob@acme.io — already exists, logged in`);
  }

  console.log("[seed] pending invites (no-ops if they already exist)");
  await tryInviteToOrg(alice, acme.id, "carol@partner.dev");
  await tryInviteToBilling(alice, alice.whoami.billingAccountId, "dave@late.io", "ROLE_MEMBER");

  const manifest = {
    base: BASE,
    password: PASSWORD,
    users: {
      alice: { email: alice.email, token: alice.token, whoami: alice.whoami },
      bob:   { email: bob.email,   token: bob.token,   whoami: bob.whoami },
      carol: { email: carol.email, token: carol.token, whoami: carol.whoami },
      dave:  { email: dave.email,  token: dave.token,  whoami: dave.whoami },
    },
    org: { id: acme.id, name: acme.displayName },
  };
  await writeFile(".seed.json", JSON.stringify(manifest, null, 2));
  console.log("\n[seed] done. .seed.json written.");
  console.log("\nLogin as anyone via:");
  console.log(`  email: <alice|bob|carol|dave>@…`);
  console.log(`  password: ${PASSWORD}`);
  console.log("\nOr drop their session into the browser:");
  console.log(`  localStorage.setItem('wm.session', JSON.stringify({`);
  console.log(`    token: '<token>', whoami: {…},`);
  console.log(`  })); location.reload();`);
}

main().catch((err) => {
  console.error(`[seed] FAILED: ${err.message}`);
  process.exit(1);
});
