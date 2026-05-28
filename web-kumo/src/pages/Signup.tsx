import { useState, type FormEvent } from "react";
import { Link, useNavigate, useSearchParams } from "react-router-dom";
import { Banner, Button, Input, SensitiveInput, Text } from "@cloudflare/kumo";
import { useAuth } from "../auth";
import { authClient, errorMessage } from "../client";
import { AuthHero } from "../components/AuthHero";

export function Signup() {
  const { setSession } = useAuth();
  const nav = useNavigate();
  const [params] = useSearchParams();
  const inviteToken = params.get("invite") ?? undefined;

  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      const res = await authClient.signup({
        email,
        password,
        inviteToken,
      });
      if (!res.whoami) throw new Error("signup response missing whoami");
      setSession(res.sessionToken, res.whoami);
      nav("/", { replace: true });
    } catch (err) {
      setError(errorMessage(err, "signup failed"));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="page">
      <AuthHero
        eyebrow="Session / Provision"
        title="Create account."
        lede="A fresh billing scope is minted on first sign-in. Org scopes are added later via invitation."
      />

      <form onSubmit={onSubmit} className="flex flex-col gap-4">
        {inviteToken && (
          <Banner variant="default">
            Enrolling against an open invitation. Membership applies once the account is minted.
          </Banner>
        )}
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
          autoComplete="new-password"
          required
          minLength={8}
          placeholder="min 8 characters"
          value={password}
          onValueChange={setPassword}
          description="Stored as a scrypt hash. Never echoed."
        />

        <div className="flex items-center justify-between gap-3 flex-wrap">
          <Button type="submit" variant="primary" disabled={busy}>
            {busy ? "Creating…" : "Create account"}
          </Button>
          <Text variant="secondary">
            Have an account?{" "}
            <Link to="/login" className="text-kumo-brand underline">
              Log in
            </Link>
          </Text>
        </div>
      </form>
    </div>
  );
}
