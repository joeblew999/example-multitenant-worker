import { useState } from "react";
import type { MembershipSummary } from "../../gen/workers/auth/v1/auth_pb.js";
import { AuthMethodKind, Role } from "../../gen/workers/auth/v1/auth_pb.js";

const roleLabel: Record<Role, string> = {
  [Role.OWNER]: "owner",
  [Role.MEMBER]: "member",
  [Role.UNSPECIFIED]: "unknown",
};
import { useAuth } from "../auth";
import { authClient, errorMessage } from "../client";

export function Dashboard() {
  const { state, setSession, refreshWhoami } = useAuth();
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);

  if (state.status !== "authenticated") return null;
  const { whoami } = state;

  const currentScopeKind = whoami.orgId ? "Organization" : "Billing account";
  const currentScopeName = whoami.orgId
    ? whoami.orgMemberships.find((m) => m.scopeId === whoami.orgId)?.displayName || whoami.orgId
    : whoami.billingMemberships.find((m) => m.scopeId === whoami.billingAccountId)?.displayName ||
      whoami.billingAccountId;

  async function switchTo(req: { billingAccountId?: string; orgId?: string }, key: string) {
    setError(null);
    setBusyId(key);
    try {
      const res = await authClient.switchContext(req);
      if (!res.whoami) throw new Error("switch response missing whoami");
      setSession(res.sessionToken, res.whoami);
    } catch (e) {
      setError(errorMessage(e, "switch failed"));
    } finally {
      setBusyId(null);
    }
  }

  return (
    <div className="page">
      <section className="hero">
        <span className="eyebrow">Current scope · {currentScopeKind}</span>
        <h1 className="display">{currentScopeName}</h1>
        <p className="lede">
          Operating as <strong>{whoami.email}</strong> with role{" "}
          <span className="chip active">{roleLabel[whoami.role]}</span>. Switch scopes below — every pivot
          mints a fresh, narrowly-scoped session token.
        </p>
      </section>

      <section>
        <div className="section-head">
          <h2>Account</h2>
          <button
            type="button"
            className="ghost"
            onClick={() => {
              refreshWhoami().catch((e) => setError(errorMessage(e, "refresh failed")));
            }}
          >
            Refresh
          </button>
        </div>

        {error && (
          <p className="status-line error" style={{ marginBottom: 16 }}>
            {error}
          </p>
        )}

        <dl className="kv">
          <div className="row">
            <dt>User ID</dt>
            <dd>
              <span className="mono">{whoami.userId}</span>
            </dd>
          </div>
          <div className="row">
            <dt>Email</dt>
            <dd>
              {whoami.email}
              {!whoami.emailVerified && <span className="secondary"> · unverified</span>}
            </dd>
          </div>
          <div className="row">
            <dt>Auth method</dt>
            <dd>
              <span className="chip">
                {whoami.authMethod?.kind === AuthMethodKind.SSO
                  ? `sso${whoami.authMethod.idpId ? `:${whoami.authMethod.idpId}` : ""}`
                  : "password"}
              </span>
            </dd>
          </div>
          <div className="row">
            <dt>Active role</dt>
            <dd>
              <span className="chip active">{roleLabel[whoami.role]}</span>
            </dd>
          </div>
          <div className="row">
            <dt>Billing scope</dt>
            <dd>
              <span className="mono">{whoami.billingAccountId}</span>
            </dd>
          </div>
          <div className="row">
            <dt>Org scope</dt>
            <dd>
              {whoami.orgId ? (
                <span className="mono">{whoami.orgId}</span>
              ) : (
                <span className="secondary">— operating at billing scope</span>
              )}
            </dd>
          </div>
        </dl>
      </section>

      <Memberships
        title="Billing accounts"
        subtitle="Tenancy roots."
        items={whoami.billingMemberships}
        currentScopeId={whoami.billingAccountId}
        busyId={busyId}
        onSelect={(id) => switchTo({ billingAccountId: id }, `billing:${id}`)}
        keyPrefix="billing"
      />

      <Memberships
        title="Organizations"
        subtitle="Sub-scopes nested beneath a billing root."
        items={whoami.orgMemberships}
        currentScopeId={whoami.orgId}
        busyId={busyId}
        onSelect={(id) => switchTo({ orgId: id }, `org:${id}`)}
        keyPrefix="org"
      />
    </div>
  );
}

type MembershipsProps = {
  title: string;
  subtitle: string;
  items: MembershipSummary[];
  currentScopeId: string;
  busyId: string | null;
  onSelect: (scopeId: string) => void;
  keyPrefix: string;
};

function Memberships({
  title,
  subtitle,
  items,
  currentScopeId,
  busyId,
  onSelect,
  keyPrefix,
}: MembershipsProps) {
  return (
    <section>
      <div className="section-head">
        <h2>{title}</h2>
        <span className="meta">
          {items.length.toString().padStart(2, "0")} ·{" "}
          {items.length === 1 ? "membership" : "memberships"}
        </span>
      </div>
      <p className="section-sub">{subtitle}</p>

      {items.length === 0 ? (
        <div className="list-empty">No memberships</div>
      ) : (
        <ul className="list">
          {items.map((m) => {
            const k = `${keyPrefix}:${m.scopeId}`;
            const current = m.scopeId === currentScopeId;
            return (
              <li key={m.scopeId} className={current ? "is-current" : undefined}>
                <div className="body">
                  <span className="name">{m.displayName || m.scopeId}</span>
                  <span className="meta">
                    <span className="role">{m.role}</span>
                    <span className="id">{m.scopeId}</span>
                    {current && <span className="live">Active</span>}
                  </span>
                </div>
                <button
                  type="button"
                  className="secondary"
                  disabled={current || busyId !== null}
                  onClick={() => onSelect(m.scopeId)}
                >
                  {busyId === k ? "Switching…" : current ? "Active" : "Switch"}
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
