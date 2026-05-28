import type { SeedScenario } from "./types";

/**
 * Editorial / Acme Corp demo accounts. Must match the users created
 * by scripts/seed/scenario.editorial.mjs.
 */
export const EDITORIAL: SeedScenario = {
  name: "editorial",
  canonicalTheme: "editorial",
  password: "demo-password-123",
  accounts: [
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
  ],
};
