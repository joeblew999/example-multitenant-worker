import { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import type { Invitation } from "../../gen/workers/invitation/v1/invitation_pb.js";
import { InvitationStatus, ScopeKind } from "../../gen/workers/invitation/v1/invitation_pb.js";
import { timestampMs } from "@bufbuild/protobuf/wkt";
import { useAuth } from "../auth";
import { authClient, errorMessage, invitationClient } from "../client";

type LoadState =
  | { kind: "loading" }
  | { kind: "loaded"; invitation: Invitation }
  | { kind: "error"; message: string };

export function AcceptInvite() {
  const { token } = useParams<{ token: string }>();
  const { state, setSession, refreshWhoami } = useAuth();
  const nav = useNavigate();

  const [load, setLoad] = useState<LoadState>({ kind: "loading" });
  const [busy, setBusy] = useState<"accept" | "decline" | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);

  const sessionEmail = state.status === "authenticated" ? state.whoami.email : null;

  useEffect(() => {
    if (!token) {
      setLoad({ kind: "error", message: "missing invite token" });
      return;
    }
    if (state.status === "loading") return;
    if (load.kind === "loaded") return;

    let cancelled = false;
    setLoad({ kind: "loading" });
    invitationClient
      .getInvitation({ token })
      .then((res) => {
        if (cancelled) return;
        if (!res.invitation) {
          setLoad({ kind: "error", message: "invitation not found" });
          return;
        }
        setLoad({ kind: "loaded", invitation: res.invitation });
      })
      .catch((e) => {
        if (cancelled) return;
        setLoad({ kind: "error", message: errorMessage(e, "could not load invitation") });
      });
    return () => {
      cancelled = true;
    };
  }, [token, state.status]);

  if (load.kind === "loading") {
    return (
      <div className="page">
        <p className="loading">Resolving invitation</p>
      </div>
    );
  }
  if (load.kind === "error") {
    return (
      <div className="page">
        <section className="hero">
          <span className="eyebrow">Invitation</span>
          <h1 className="display sm">Invitation unavailable.</h1>
        </section>
        <p className="status-line error">{load.message}</p>
        <p className="status-line">
          <Link to="/">Back to dashboard</Link>
        </p>
      </div>
    );
  }

  const inv = load.invitation;
  const expiresAt = inv.expiresAt ? timestampMs(inv.expiresAt) : 0;
  const expired = expiresAt > 0 && expiresAt < Date.now();
  const inactive = inv.status !== InvitationStatus.PENDING || expired;
  const emailMismatch =
    sessionEmail !== null && inv.email.toLowerCase() !== sessionEmail.toLowerCase();
  const scopeName = inv.scopeDisplayName || inv.scopeId;

  async function onAccept() {
    if (!token) return;
    setActionError(null);
    setBusy("accept");
    try {
      const accepted = await invitationClient.acceptInvitation({ token });
      const switchReq =
        accepted.scopeKind === ScopeKind.ORG
          ? { orgId: accepted.scopeId }
          : { billingAccountId: accepted.scopeId };
      try {
        const switched = await authClient.switchContext(switchReq);
        if (switched.whoami) setSession(switched.sessionToken, switched.whoami);
      } catch {
        // Switching is a convenience — accept already succeeded. Refresh
        // whoami so the new membership at least shows up in the dashboard.
        await refreshWhoami().catch(() => {});
      }
      nav("/", { replace: true });
    } catch (e) {
      setActionError(errorMessage(e, "accept failed"));
    } finally {
      setBusy(null);
    }
  }

  async function onDecline() {
    if (!token) return;
    setActionError(null);
    setBusy("decline");
    try {
      await invitationClient.declineInvitation({ token });
      nav("/", { replace: true });
    } catch (e) {
      setActionError(errorMessage(e, "decline failed"));
    } finally {
      setBusy(null);
    }
  }

  return (
    <div className="page">
      <section className="hero">
        <span className="eyebrow">
          Invitation · {inv.scopeKind} · status {inv.status}
          {expired ? " · expired" : ""}
        </span>
        <h1 className="display">{scopeName}</h1>
        <p className="lede">
          You've been invited to join <strong>{scopeName}</strong> as{" "}
          <span className="chip active">{inv.role}</span>. Accepting mints a membership and pivots
          your session into it.
        </p>
      </section>

      <section>
        <div className="section-head">
          <h2>Details</h2>
          <span className="meta">
            {expiresAt > 0 ? `Expires ${new Date(expiresAt).toLocaleString()}` : "No expiry"}
          </span>
        </div>

        <dl className="kv">
          <div className="row">
            <dt>Scope</dt>
            <dd>
              <strong>{scopeName}</strong> <span className="chip">{inv.scopeKind}</span>
            </dd>
          </div>
          <div className="row">
            <dt>Role offered</dt>
            <dd>
              <span className="chip active">{inv.role}</span>
            </dd>
          </div>
          <div className="row">
            <dt>Issued to</dt>
            <dd>
              <span className="mono">{inv.email}</span>
            </dd>
          </div>
          <div className="row">
            <dt>Status</dt>
            <dd>
              {inv.status}
              {expired && <span className="secondary"> · expired</span>}
            </dd>
          </div>
          {inv.requiredIdp && (
            <div className="row">
              <dt>SSO required</dt>
              <dd>
                <span className="chip">{inv.requiredIdp}</span>
              </dd>
            </div>
          )}
        </dl>
      </section>

      {actionError && <p className="status-line error">{actionError}</p>}

      <div className="row">
        <InviteActions
          inactive={inactive}
          authStatus={state.status}
          emailMismatch={emailMismatch}
          inviteEmail={inv.email}
          sessionEmail={sessionEmail}
          token={token!}
          busy={busy}
          onAccept={onAccept}
          onDecline={onDecline}
          nav={nav}
        />
      </div>
    </div>
  );
}

type InviteActionsProps = {
  inactive: boolean;
  authStatus: "loading" | "anonymous" | "authenticated";
  emailMismatch: boolean;
  inviteEmail: string;
  sessionEmail: string | null;
  token: string;
  busy: "accept" | "decline" | null;
  onAccept: () => void;
  onDecline: () => void;
  nav: ReturnType<typeof useNavigate>;
};

function InviteActions(props: InviteActionsProps) {
  const {
    inactive,
    authStatus,
    emailMismatch,
    inviteEmail,
    sessionEmail,
    token,
    busy,
    onAccept,
    onDecline,
    nav,
  } = props;

  if (inactive) {
    return <p className="status-line">This invitation is no longer accepting responses.</p>;
  }

  if (authStatus === "anonymous") {
    return (
      <>
        <button type="button" onClick={() => nav(`/signup?invite=${encodeURIComponent(token)}`)}>
          Sign up to accept
        </button>
        <button
          type="button"
          className="secondary"
          onClick={() => nav(`/login?next=${encodeURIComponent(`/invite/${token}`)}`)}
        >
          Log in to accept
        </button>
      </>
    );
  }

  if (emailMismatch) {
    return (
      <p className="status-line error">
        Issued to <strong>{inviteEmail}</strong>, but you're logged in as{" "}
        <strong>{sessionEmail}</strong>. Sign out and log in with the invited address to accept.
      </p>
    );
  }

  return (
    <>
      <button type="button" disabled={busy !== null} onClick={onAccept}>
        {busy === "accept" ? "Accepting…" : "Accept"}
      </button>
      <button type="button" className="secondary" disabled={busy !== null} onClick={onDecline}>
        {busy === "decline" ? "Declining…" : "Decline"}
      </button>
    </>
  );
}
