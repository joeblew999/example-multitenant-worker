import { useState, type FormEvent } from "react";
import { Link, useNavigate, useSearchParams } from "react-router-dom";
import { useAuth } from "../auth";
import { authClient, errorMessage } from "../client";
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
    <div className="page">
      <section className="hero">
        <span className="eyebrow">Session / Login</span>
        <h1 className="display">Log in.</h1>
        <p className="lede">
          Resume your scoped session. Tokens are minted server-side and bound to your account.
        </p>
      </section>

      <form className="form" onSubmit={onSubmit}>
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
            autoComplete="current-password"
            required
            placeholder="••••••••"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        </label>

        <div className="row between">
          <button type="submit" disabled={busy}>
            {busy ? "Authenticating…" : "Log in"}
          </button>
          <span className="status-line">
            No account? <Link to="/signup">Sign up</Link>
          </span>
        </div>

        <DevAccounts compact />
      </form>
    </div>
  );
}
