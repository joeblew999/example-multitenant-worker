import type { SeedScenario } from "./types";

/**
 * RemySports / Bangkok Suns demo accounts. Must match the users
 * created by scripts/seed/scenario.remysport.mjs.
 */
export const REMYSPORT: SeedScenario = {
  name: "remysport",
  canonicalTheme: "remysport",
  password: "demo-password-123",
  accounts: [
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
  ],
};
