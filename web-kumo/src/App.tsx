import { Navigate, Route, Routes, Link } from "react-router-dom";
import { useAuth } from "./auth";
import { Login } from "./pages/Login";
import { Signup } from "./pages/Signup";
import { Dashboard } from "./pages/Dashboard";
import { AcceptInvite } from "./pages/AcceptInvite";
import { Preview } from "./pages/Preview";
import { Members } from "./pages/Members";
import { Billing } from "./pages/Billing";
import { Invitations } from "./pages/Invitations";
import { AppShell } from "./components/AppShell";
import { PageLoading } from "./components/PageLoading";

export function App() {
  const { state } = useAuth();
  // The topbar only shows on auth pages (login/signup/accept-invite) —
  // AppShell hides it via :has(). Brand-link destination depends on
  // whether the visitor is logged in: authed → dashboard; anon → /preview
  // (the public showcase). Pointing anon visitors at "/" used to bounce
  // them back to /login via RequireAuth — a no-op trap.
  const brandHref = state.status === "authenticated" ? "/" : "/preview";

  return (
    <div className="shell">
      {/* Minimal topbar — just the brand. Hidden on AppShell pages
       * (sidebar already shows the brand) via layout-fixes.css.
       * The old SystemStatus clock + duplicate auth nav + footer are
       * gone — they didn't earn their real estate. */}
      <header className="topbar">
        <Link to={brandHref} className="brand">
          <span className="dot" aria-hidden />
          <span>Multitenant</span>
        </Link>
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
          <Route
            path="/members"
            element={
              <RequireAuth>
                <AppShell>
                  <Members />
                </AppShell>
              </RequireAuth>
            }
          />
          <Route
            path="/billing"
            element={
              <RequireAuth>
                <AppShell>
                  <Billing />
                </AppShell>
              </RequireAuth>
            }
          />
          <Route
            path="/invitations"
            element={
              <RequireAuth>
                <AppShell>
                  <Invitations />
                </AppShell>
              </RequireAuth>
            }
          />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </main>
    </div>
  );
}

function RequireAuth({ children }: { children: React.ReactNode }) {
  const { state } = useAuth();
  if (state.status === "loading") return <PageLoading label="Resolving session" />;
  if (state.status === "anonymous") return <Navigate to="/login" replace />;
  return <>{children}</>;
}

function RequireAnon({ children }: { children: React.ReactNode }) {
  const { state } = useAuth();
  if (state.status === "loading") return <PageLoading label="Resolving session" />;
  if (state.status === "authenticated") return <Navigate to="/" replace />;
  return <>{children}</>;
}
