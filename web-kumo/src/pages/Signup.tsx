import { useState, type FormEvent } from "react";
import { Link, useNavigate, useSearchParams } from "react-router-dom";
import { useAuth } from "../auth";
import { authClient, errorMessage } from "../client";

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
      <section className="hero">
        <span className="eyebrow">Session / Provision</span>
        <h1 className="display">Create account.</h1>
        <p className="lede">
          A fresh billing scope is minted on first sign-in. Org scopes are added later via
          invitation.
        </p>
      </section>

      <form className="form" onSubmit={onSubmit}>
        {inviteToken && (
          <p className="status-line notice">
            Enrolling against an open invitation. Membership applies once the account is minted.
          </p>
        )}
        {error && <p className="status-line error">{error}</p>}

        <label className="field">
          <span className="label">Email</span>
          <input
            type="email"
            autoComplete="email"
            required
            placeholder="you@example.com"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
        </label>

        <label className="field">
          <span className="label">Password</span>
          <input
            type="password"
            autoComplete="new-password"
            required
            minLength={8}
            placeholder="min 8 characters"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
          <span className="hint">Stored as scrypt hash. Never echoed.</span>
        </label>

        <div className="row between">
          <button type="submit" disabled={busy}>
            {busy ? "Creating…" : "Create account"}
          </button>
          <span className="status-line">
            Have an account? <Link to="/login">Log in</Link>
          </span>
        </div>
      </form>
    </div>
  );
}
