import { useEffect, useState } from "react";
import { Navigate, Route, Routes, Link, useLocation } from "react-router-dom";
import { useAuth } from "./auth";
import { Login } from "./pages/Login";
import { Signup } from "./pages/Signup";
import { Dashboard } from "./pages/Dashboard";
import { AcceptInvite } from "./pages/AcceptInvite";
import { Preview } from "./pages/Preview";
import { AppShell } from "./components/AppShell";

export function App() {
  const { state, logout } = useAuth();
  const loc = useLocation();

  return (
    <div className="shell">
      <header className="topbar">
        <Link to="/" className="brand">
          <span className="dot" aria-hidden />
          <span>Multitenant</span>
          <span className="ver">v0.1.0</span>
        </Link>

        <nav className="topnav">
          <SystemStatus />
          <span className="sep" aria-hidden />
          {state.status === "authenticated" ? (
            <>
              <Link to="/" aria-current={loc.pathname === "/" ? "page" : undefined}>
                Dashboard
              </Link>
              <button className="ghost" type="button" onClick={logout}>
                Sign out
              </button>
            </>
          ) : (
            <>
              <Link to="/login" aria-current={loc.pathname === "/login" ? "page" : undefined}>
                Log in
              </Link>
              <Link to="/signup" aria-current={loc.pathname === "/signup" ? "page" : undefined}>
                Sign up
              </Link>
            </>
          )}
        </nav>
      </header>

      <main>
        <Routes>
          <Route
            path="/login"
            element={
              <RequireAnon>
                <Login />
              </RequireAnon>
            }
          />
          <Route
            path="/signup"
            element={
              <RequireAnon>
                <Signup />
              </RequireAnon>
            }
          />
          <Route path="/invite/:token" element={<AcceptInvite />} />
          <Route
            path="/preview"
            element={
              <AppShell>
                <Preview />
              </AppShell>
            }
          />
          <Route
            path="/"
            element={
              <RequireAuth>
                <AppShell>
                  <Dashboard />
                </AppShell>
              </RequireAuth>
            }
          />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </main>

      <footer className="footer-caption">
        <span>Multitenant // Account</span>
        <span>Connect-RPC · Cloudflare Workers</span>
      </footer>
    </div>
  );
}

function SystemStatus() {
  const [now, setNow] = useState(() => formatClock(new Date()));
  useEffect(() => {
    let id: number;
    function tick() {
      setNow(formatClock(new Date()));
      id = window.setTimeout(tick, 1000 - (Date.now() % 1000));
    }
    id = window.setTimeout(tick, 1000 - (Date.now() % 1000));
    return () => window.clearTimeout(id);
  }, []);
  return (
    <span className="status" title="Live · UTC">
      <span className="signal" aria-hidden />
      <span>Live · {now}</span>
    </span>
  );
}

function formatClock(d: Date): string {
  const hh = String(d.getUTCHours()).padStart(2, "0");
  const mm = String(d.getUTCMinutes()).padStart(2, "0");
  const ss = String(d.getUTCSeconds()).padStart(2, "0");
  return `${hh}:${mm}:${ss}Z`;
}

function RequireAuth({ children }: { children: React.ReactNode }) {
  const { state } = useAuth();
  if (state.status === "loading") return <p className="loading">Resolving session</p>;
  if (state.status === "anonymous") return <Navigate to="/login" replace />;
  return <>{children}</>;
}

function RequireAnon({ children }: { children: React.ReactNode }) {
  const { state } = useAuth();
  if (state.status === "loading") return <p className="loading">Resolving session</p>;
  if (state.status === "authenticated") return <Navigate to="/" replace />;
  return <>{children}</>;
}
