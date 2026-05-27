/**
 * Click-to-sign-in panel for the 4 seeded demo accounts.
 *
 * Source-of-truth for the credentials is `scripts/seed.mjs` — when you
 * change the seed, update this hardcoded list to match.
 *
 * Hidden by the VITE_SHOW_TEST_ACCOUNTS env var:
 *   - undefined / "true"  →  shown   (default for this demo deploy)
 *   - "false"             →  hidden  (set in .env.production when this
 *                                     stack is ported to a real project)
 */

import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { Badge, Banner, LayerCard, Text } from "@cloudflare/kumo";
import { authClient, errorMessage } from "../client";
import { useAuth } from "../auth";

const PASSWORD = "demo-password-123";

interface DemoAccount {
  email: string;
  label: string;
  scenario: string;
  landAt: string;
  badge: "orange" | "blue" | "purple" | "teal";
}

const ACCOUNTS: DemoAccount[] = [
  {
    email: "alice@acme.io",
    label: "Alice (owner of 6 orgs)",
    scenario: "Owns Acme + 5 other orgs; multi-org scope-switcher demo",
    landAt: "/",
    badge: "orange",
  },
  {
    email: "bob@acme.io",
    label: "Bob (multi-role)",
    scenario: "Member of Acme/Engineering, owner of Marketing — mixed-role rows",
    landAt: "/",
    badge: "blue",
  },
  {
    email: "carol@partner.dev",
    label: "Carol (5 pending invites)",
    scenario: "Lots of pending invites across different orgs — invitations volume",
    landAt: "/invitations",
    badge: "purple",
  },
  {
    email: "dave@late.io",
    label: "Dave (pending billing invite)",
    scenario: "One pending invite to alice's billing scope",
    landAt: "/invitations",
    badge: "teal",
  },
];

function isEnabled(): boolean {
  const flag = (import.meta as { env?: { VITE_SHOW_TEST_ACCOUNTS?: string } }).env?.VITE_SHOW_TEST_ACCOUNTS;
  if (flag === "false") return false;
  return true;
}

export function DevAccounts({ compact = false }: { compact?: boolean }) {
  const { setSession } = useAuth();
  const nav = useNavigate();
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  if (!isEnabled()) return null;

  async function signInAs(acc: DemoAccount) {
    setBusy(acc.email);
    setError(null);
    try {
      const res = await authClient.login({ email: acc.email, password: PASSWORD });
      if (!res.whoami) throw new Error("login response missing whoami");
      setSession(res.sessionToken, res.whoami);
      nav(acc.landAt, { replace: true });
    } catch (err) {
      setError(errorMessage(err, `failed to sign in as ${acc.email}`));
      setBusy(null);
    }
  }

  if (compact) {
    return (
      <p className="status-line">
        Tester? <a href="/preview">View demo accounts →</a>
      </p>
    );
  }

  return (
    <LayerCard className="p-6">
      <div className="mb-4">
        <Text as="h2" variant="heading2">Demo accounts</Text>
        <p className="mt-1 text-kumo-subtle text-sm">
          Click any account to sign in. Shared password:{" "}
          <code className="font-mono">{PASSWORD}</code>.
        </p>
      </div>

      {error && <Banner variant={"danger" as never}>{error}</Banner>}

      <div className="flex flex-col gap-3">
        {ACCOUNTS.map((acc) => (
          <div
            key={acc.email}
            className="flex flex-col sm:flex-row sm:items-center gap-3 p-4 rounded-lg ring ring-kumo-line"
          >
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-1">
                <Badge variant={acc.badge as never}>{acc.label}</Badge>
              </div>
              <div className="text-kumo-default font-mono text-sm truncate">{acc.email}</div>
              <p className="text-kumo-subtle text-sm mt-1">{acc.scenario}</p>
            </div>
            <button
              type="button"
              onClick={() => signInAs(acc)}
              disabled={busy !== null}
              className="shrink-0"
            >
              {busy === acc.email ? "Signing in…" : "Sign in"}
            </button>
          </div>
        ))}
      </div>

      <p className="mt-4 text-kumo-subtle text-xs">
        Test accounts are visible because <code className="font-mono">VITE_SHOW_TEST_ACCOUNTS</code> is unset or "true".
        Set to "false" in .env.production to hide on real deployments.
      </p>
    </LayerCard>
  );
}
