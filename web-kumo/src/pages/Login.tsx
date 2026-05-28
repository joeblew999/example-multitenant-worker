import { useState, type FormEvent } from "react";
import { Link, useNavigate, useSearchParams } from "react-router-dom";
import { Banner, Button, Input, SensitiveInput, Text } from "@cloudflare/kumo";
import { useAuth } from "../auth";
import { authClient, errorMessage } from "../client";
import { AuthHero } from "../components/AuthHero";
import { DevAccounts } from "../components/DevAccounts";

// Only honor a `next` redirect if it's an internal, absolute path. Stops
// `?next=https://evil.example/...` from bouncing the user offsite.
function safeNext(raw: string | null): string {
  if (!raw) return "/";
  if (!raw.startsWith("/") || raw.startsWith("//")) return "/";
  return raw;
}

export function Login() {
  const { setSession } = useAuth();
  const nav = useNavigate();
  const [params] = useSearchParams();
  const next = safeNext(params.get("next"));

  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      const res = await authClient.login({ email, password });
      if (!res.whoami) throw new Error("login response missing whoami");
      setSession(res.sessionToken, res.whoami);
      nav(next, { replace: true });
    } catch (err) {
      setError(errorMessage(err, "login failed"));
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <div className="page">
        <AuthHero
          eyebrow="Session / Login"
          title="Log in."
          lede="Resume your scoped session. Tokens are minted server-side and bound to your account."
        />

        <form onSubmit={onSubmit} className="flex flex-col gap-4">
          {error && <Banner variant="error">{error}</Banner>}

          <Input
            label="Email"
            type="email"
            autoComplete="email"
            required
            placeholder="you@example.com"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />

          <SensitiveInput
            label="Password"
            autoComplete="current-password"
            required
            placeholder="••••••••"
            value={password}
            onValueChange={setPassword}
          />

          <div className="flex items-center justify-between gap-3 flex-wrap">
            <Button type="submit" variant="primary" disabled={busy}>
              {busy ? "Authenticating…" : "Log in"}
            </Button>
            <Text variant="secondary">
              No account?{" "}
              <Link to="/signup" className="text-kumo-brand underline">
                Sign up
              </Link>
            </Text>
          </div>
        </form>
      </div>

      {/* Demo accounts panel — rendered as a sibling so it can exceed the
        * 480px .page card width. Hidden in production via VITE_SHOW_TEST_ACCOUNTS. */}
      <div className="w-full max-w-2xl mx-auto">
        <DevAccounts />
      </div>
    </>
  );
}
