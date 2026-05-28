/**
 * Shared seed helpers — Connect RPC calls + idempotent
 * ensureUser/ensureOrg/tryInvite/inviteAndJoin.
 *
 * Scenarios import these and compose their own narrative. The helpers
 * are agnostic to which scenario is running.
 */

process.env.NODE_TLS_REJECT_UNAUTHORIZED = "0";
process.removeAllListeners("warning");

/**
 * @param {string} base
 * @param {string} service
 * @param {string} method
 * @param {object} body
 * @param {string} [token]
 */
export async function rpc(base, service, method, body, token) {
  const url = `${base}/workers.${service}/${method}`;
  const res = await fetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    body: JSON.stringify(body ?? {}),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`${service}/${method} → ${res.status}: ${text}`);
  }
  return res.json();
}

/**
 * Build a bound helpers object so scenario files don't have to pass
 * base + password to every call. Each scenario gets one of these.
 */
export function makeHelpers(base, password) {
  const call = (service, method, body, token) => rpc(base, service, method, body, token);

  async function login(email) {
    try {
      const r = await call("auth.v1.AuthService", "Login", { email, password });
      return { email, token: r.sessionToken, whoami: r.whoami };
    } catch {
      return null;
    }
  }

  async function ensureUser(email) {
    const existing = await login(email);
    if (existing) return existing;
    const r = await call("auth.v1.AuthService", "Signup", { email, password });
    return { email, token: r.sessionToken, whoami: r.whoami };
  }

  async function ensureUserWithInvite(email, inviteToken) {
    const existing = await login(email);
    if (existing) return existing;
    const r = await call("auth.v1.AuthService", "Signup", {
      email, password, inviteToken,
    });
    return { email, token: r.sessionToken, whoami: r.whoami };
  }

  async function ensureOrg(user, displayName) {
    const billingAccountId = user.whoami.billingAccountId;
    try {
      const list = await call("org.v1.OrgService", "ListOrganizations",
        { billingAccountId }, user.token);
      const hit = (list.organizations || []).find(
        (o) => o.displayName === displayName && !o.personal
      );
      if (hit) return hit;
    } catch {}
    const r = await call("org.v1.OrgService", "CreateOrganization",
      { billingAccountId, displayName }, user.token);
    return r.organization;
  }

  async function tryInviteToOrg(user, orgId, email, role = "ROLE_MEMBER") {
    try {
      const r = await call("org.v1.OrgService", "InviteMember",
        { orgId, email, role }, user.token);
      return { invitationId: r.invitationId, token: r.inviteTokenForDemo };
    } catch {
      return null;
    }
  }

  async function tryInviteToBilling(user, billingAccountId, email, role = "ROLE_MEMBER") {
    try {
      const r = await call("billing.v1.BillingService", "InviteMember",
        { billingAccountId, email, role }, user.token);
      return { invitationId: r.invitationId, token: r.inviteTokenForDemo };
    } catch {
      return null;
    }
  }

  /**
   * Invite + auto-accept by signup — for batch member-population.
   * `role` controls the membership granted on accept. Defaults to MEMBER.
   * Pass "ROLE_OWNER" for head coaches / organizers / etc.
   */
  async function inviteAndJoin(inviter, orgId, email, role = "ROLE_MEMBER") {
    const existing = await login(email);
    if (existing) return existing;
    const inv = await tryInviteToOrg(inviter, orgId, email, role);
    if (!inv) return await ensureUser(email);
    return await ensureUserWithInvite(email, inv.token);
  }

  async function healthCheck() {
    const health = await fetch(`${base}/healthz`);
    if (!health.ok) throw new Error(`worker not reachable at ${base}/healthz`);
  }

  return {
    base, password,
    call, login, ensureUser, ensureUserWithInvite,
    ensureOrg, tryInviteToOrg, tryInviteToBilling, inviteAndJoin, healthCheck,
  };
}
