/**
 * RemySports demo scenario — basketball club tenancy.
 *
 * Maps the multitenant data model onto the basketball-club domain
 * defined by remy-sport-biz (joeblew999/remy-sport-biz):
 *
 *   billing account  ←→  club / franchise paying for the platform
 *   organization     ←→  team within the club (Senior A, U18 Boys, etc.)
 *   role             ←→  coach (OWNER) / player or staff (MEMBER)
 *
 * Personas keep the same structural variety as editorial so the GUI
 * gets exercised the same way (multi-team, pending-invites volume,
 * long-name overflow):
 *
 *   - coach (alice equivalent) owns the club + 5 teams
 *   - captain (bob) is multi-team: member of two squads, head of one
 *   - scout (carol) gets 5 pending team-trial invites
 *   - manager (dave) has 1 pending club-staff invite
 *
 * All emails use the `.example` TLD per RFC 6761 so the seed is safe
 * to run against production.
 */

export const SCENARIO_NAME = "remysport";
export const DESCRIPTION = "Bangkok Suns basketball club — 5 teams, multi-team captain, scout invites";

export async function run(h) {
  console.log("[seed:remysport] core users");
  const coach   = await h.ensureUser("coach@bangkok-suns.example");
  const scout   = await h.ensureUser("scout@asia-league.example");
  const manager = await h.ensureUser("manager@suns-academy.example");

  console.log("[seed:remysport] coach's teams (5 + long-name)");
  const club   = await h.ensureOrg(coach, "Bangkok Suns");
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
  // Unicode display test — Thai script in display path
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
