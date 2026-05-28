import { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { timestampMs } from "@bufbuild/protobuf/wkt";
import { Badge, Banner, Button, Text } from "@cloudflare/kumo";
import type { Invitation } from "../../gen/workers/invitation/v1/invitation_pb.js";
import {
  InvitationStatus,
  Role,
  ScopeKind,
} from "../../gen/workers/invitation/v1/invitation_pb.js";
import { useAuth } from "../auth";
import { authClient, errorMessage, invitationClient } from "../client";
import { AuthHero } from "../components/AuthHero";
import { KvList } from "../components/KvList";
import { PageLoading } from "../components/PageLoading";

const SCOPE_LABEL: Record<ScopeKind, string> = {
  [ScopeKind.UNSPECIFIED]: "—",
  [ScopeKind.BILLING]: "Billing",
  [ScopeKind.ORG]: "Organization",
};

const ROLE_LABEL: Record<Role, string> = {
  [Role.UNSPECIFIED]: "—",
  [Role.OWNER]: "owner",
  [Role.MEMBER]: "member",
};

const STATUS_LABEL: Record<InvitationStatus, string> = {
  [InvitationStatus.UNSPECIFIED]: "—",
  [InvitationStatus.PENDING]: "pending",
  [InvitationStatus.ACCEPTED]: "accepted",
  [InvitationStatus.DECLINED]: "declined",
  [InvitationStatus.REVOKED]: "revoked",
  [InvitationStatus.EXPIRED]: "expired",
};

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
        <PageLoading label="Resolving invitation" />
      </div>
    );
  }

  if (load.kind === "error") {
    return (
      <div className="page">
        <AuthHero eyebrow="Invitation" title="Invitation unavailable." />
        <Banner variant="error">{load.message}</Banner>
        <Text variant="secondary">
          <Link to="/" className="text-kumo-brand underline">
            Back to dashboard
          </Link>
        </Text>
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
  const roleBadgeVariant = inv.role === Role.OWNER ? "orange" : "neutral";

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

  const eyebrow = `Invitation · ${SCOPE_LABEL[inv.scopeKind]} · ${STATUS_LABEL[inv.status]}${
    expired ? " · expired" : ""
  }`;

  return (
    <div className="page">
      <AuthHero
        eyebrow={eyebrow}
        title={scopeName}
        lede={
          <>
            You've been invited to join <strong>{scopeName}</strong> as{" "}
            <Badge variant={roleBadgeVariant}>{ROLE_LABEL[inv.role]}</Badge>. Accepting
            mints a membership and pivots your session into it.
          </>
        }
      />

      <section className="flex flex-col gap-3">
        <div className="flex items-baseline justify-between gap-3 flex-wrap">
          <Text as="h2" variant="heading3">Details</Text>
          <span className="text-sm text-kumo-subtle">
            {expiresAt > 0 ? `Expires ${new Date(expiresAt).toLocaleString()}` : "No expiry"}
          </span>
        </div>

        <KvList
          dtWidth="minmax(100px, 140px)"
          rows={[
            {
              k: "Scope",
              v: (
                <span className="inline-flex items-center gap-2">
                  <strong>{scopeName}</strong>
                  <Badge variant="neutral">{SCOPE_LABEL[inv.scopeKind]}</Badge>
                </span>
              ),
            },
            {
              k: "Role offered",
              v: <Badge variant={roleBadgeVariant}>{ROLE_LABEL[inv.role]}</Badge>,
            },
            {
              k: "Issued to",
              v: <code className="font-mono text-sm">{inv.email}</code>,
            },
            {
              k: "Status",
              v: (
                <span>
                  {STATUS_LABEL[inv.status]}
                  {expired && <span className="ml-2 text-kumo-subtle">· expired</span>}
                </span>
              ),
            },
            ...(inv.requiredIdp
              ? [
                  {
                    k: "SSO required",
                    v: <Badge variant="neutral">{inv.requiredIdp}</Badge>,
                  },
                ]
              : []),
          ]}
        />
      </section>

      {actionError && <Banner variant="error">{actionError}</Banner>}

      <div className="flex gap-3 flex-wrap">
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
    return (
      <Banner variant="default">This invitation is no longer accepting responses.</Banner>
    );
  }

  if (authStatus === "anonymous") {
    return (
      <>
        <Button
          variant="primary"
          onClick={() => nav(`/signup?invite=${encodeURIComponent(token)}`)}
        >
          Sign up to accept
        </Button>
        <Button
          variant="secondary"
          onClick={() => nav(`/login?next=${encodeURIComponent(`/invite/${token}`)}`)}
        >
          Log in to accept
        </Button>
      </>
    );
  }

  if (emailMismatch) {
    return (
      <Banner variant="error">
        Issued to <strong>{inviteEmail}</strong>, but you're logged in as{" "}
        <strong>{sessionEmail}</strong>. Sign out and log in with the invited address to
        accept.
      </Banner>
    );
  }

  return (
    <>
      <Button variant="primary" disabled={busy !== null} onClick={onAccept}>
        {busy === "accept" ? "Accepting…" : "Accept"}
      </Button>
      <Button variant="secondary" disabled={busy !== null} onClick={onDecline}>
        {busy === "decline" ? "Declining…" : "Decline"}
      </Button>
    </>
  );
}
