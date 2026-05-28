/**
 * Click-to-sign-in panel for the active seed scenario's demo accounts.
 *
 * Lives on /login alongside the credential form. Source of truth is
 * `scenarios/<active>/scenario.mjs` (chosen by VITE_SEED_SCENARIO).
 * The backend seed must create matching users; ACCOUNT emails on both
 * sides have to stay in sync.
 *
 * ## Naming convention: all seed emails use the `.example` TLD
 *
 * Per RFC 6761 the `.example` TLD is permanently reserved for
 * documentation and never resolves on the public internet. Real
 * customers can never collide with these test accounts, so devs can
 * log into prod as alice@acme.example (or coach@bangkok-suns.example,
 * etc.) without shadowing real users.
 *
 * ## Hidden by the VITE_SHOW_TEST_ACCOUNTS env var
 *
 *   - undefined / "true"  →  shown   (default for demo deploys)
 *   - "false"             →  hidden  (set in .env.production)
 */

import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { Badge, Banner, Button, LayerCard, Text } from "@cloudflare/kumo";
import { authClient, errorMessage } from "../client";
import { useAuth } from "../auth";
import { devAccountsEnabled } from "../dev-flags";
import { ACTIVE_SCENARIO, type DemoAccount } from "../seed-scenarios";

export function DevAccounts() {
  const { setSession } = useAuth();
  const nav = useNavigate();
  const [busy, setBusy] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  if (!devAccountsEnabled()) return null;

  async function signInAs(acc: DemoAccount) {
    setBusy(acc.email);
    setError(null);
    try {
      const res = await authClient.login({
        email: acc.email,
        password: ACTIVE_SCENARIO.PASSWORD,
      });
      if (!res.whoami) throw new Error("login response missing whoami");
      setSession(res.sessionToken, res.whoami);
      nav(acc.landAt, { replace: true });
    } catch (err) {
      setError(errorMessage(err, `failed to sign in as ${acc.email}`));
      setBusy(null);
    }
  }

  return (
    <LayerCard className="p-6">
      <div className="mb-4">
        <Text as="h2" variant="heading2">Demo accounts</Text>
        <p className="mt-1 text-kumo-subtle text-sm">
          Scenario:{" "}
          <code className="font-mono">{ACTIVE_SCENARIO.SCENARIO_NAME}</code>.
          Click any account to sign in. Shared password:{" "}
          <code className="font-mono">{ACTIVE_SCENARIO.PASSWORD}</code>.
        </p>
      </div>

      {error && <Banner variant="error">{error}</Banner>}

      <div className="flex flex-col gap-3">
        {ACTIVE_SCENARIO.ACCOUNTS.map((acc) => (
          <div
            key={acc.email}
            className="flex flex-col sm:flex-row sm:items-center gap-3 p-4 rounded-lg ring ring-kumo-line"
          >
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-1">
                <Badge variant={acc.badge}>{acc.label}</Badge>
              </div>
              <div className="text-kumo-default font-mono text-sm truncate">{acc.email}</div>
              <p className="text-kumo-subtle text-sm mt-1">{acc.scenario}</p>
            </div>
            <Button
              variant="secondary"
              onClick={() => signInAs(acc)}
              disabled={busy !== null}
              className="shrink-0"
            >
              {busy === acc.email ? "Signing in…" : "Sign in"}
            </Button>
          </div>
        ))}
      </div>

      <p className="mt-4 text-kumo-subtle text-xs">
        Switch demos by setting <code className="font-mono">VITE_SEED_SCENARIO</code> at
        build time (e.g. <code className="font-mono">VITE_SEED_SCENARIO=remysport pnpm dev</code>).
        That flips this list AND the canonical theme (via
        <code className="font-mono"> index.html</code>'s
        <code className="font-mono"> %VITE_SEED_SCENARIO%</code> substitution).
      </p>
    </LayerCard>
  );
}
