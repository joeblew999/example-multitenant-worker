/**
 * Editorial demo scenario — Acme Corp multi-org tenancy showcase.
 *
 * Scenario shape (volume + role variety for GUI stress-testing):
 *   - alice owns 6 orgs under her billing account
 *   - bob is multi-role: member of Acme/Eng, owner of Marketing
 *   - carol has 5 pending invites across different orgs
 *   - dave has 1 pending billing invite
 *   - 8 minor members populate org rosters
 *   - unicode user (über@müller.example) tests font rendering
 */

export const SCENARIO_NAME = "editorial";
export const DESCRIPTION = "Acme Corp — 6 orgs, multi-role bob, pending-invites carol, unicode test";

/** Returns { users, orgs, notes } so the runner can build the manifest. */
export async function run(h) {
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
  // Unicode display test
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
      alice: "/billing → 6 orgs visible; sees long-name overflow + scope-switcher density",
      bob:   "/ → multi-role: member of Acme/Eng, owner of Marketing",
      carol: "/invitations → 5 pending across different orgs (stresses Invitations table)",
      dave:  "/invitations → 1 pending billing invite",
    },
  };
}
