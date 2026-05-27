import { useEffect, useState } from "react";
import {
  Badge,
  Banner,
  Breadcrumbs,
  Empty,
  Table,
  Text,
} from "@cloudflare/kumo";
import { PageHeader } from "../components/kumo/page-header/page-header";
import { invitationClient, errorMessage } from "../client";
import type { Invitation } from "../../gen/workers/invitation/v1/invitation_pb.js";
import {
  InvitationStatus,
  Role,
  ScopeKind,
} from "../../gen/workers/invitation/v1/invitation_pb.js";

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

function statusBadge(s: InvitationStatus) {
  switch (s) {
    case InvitationStatus.PENDING:
      return <Badge variant={"orange" as never}>pending</Badge>;
    case InvitationStatus.ACCEPTED:
      return <Badge variant={"green" as never}>accepted</Badge>;
    case InvitationStatus.DECLINED:
      return <Badge variant={"neutral" as never}>declined</Badge>;
    case InvitationStatus.REVOKED:
      return <Badge variant={"red" as never}>revoked</Badge>;
    case InvitationStatus.EXPIRED:
      return <Badge variant={"neutral" as never}>expired</Badge>;
    default:
      return <Badge variant={"neutral" as never}>unknown</Badge>;
  }
}

function relativeExpiry(ts: Invitation["expiresAt"]): string {
  if (!ts) return "—";
  const sec = Number(ts.seconds);
  const ms = sec * 1000;
  const days = Math.round((ms - Date.now()) / (1000 * 60 * 60 * 24));
  if (days < 0) return `expired ${-days}d ago`;
  if (days === 0) return "expires today";
  if (days === 1) return "expires tomorrow";
  return `${days} days left`;
}

export function Invitations() {
  const [invites, setInvites] = useState<Invitation[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    invitationClient
      .listPendingInvitations({})
      .then((res) => {
        if (!cancelled) setInvites(res.invitations);
      })
      .catch((err) => {
        if (!cancelled) setError(errorMessage(err, "failed to load invitations"));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        breadcrumbs={
          <Breadcrumbs>
            <Breadcrumbs.Link href="/">Home</Breadcrumbs.Link>
            <Breadcrumbs.Separator />
            <Breadcrumbs.Current>Invitations</Breadcrumbs.Current>
          </Breadcrumbs>
        }
        title="Invitations"
        description="Pending invites where you're the invitee. Accept/decline happens via the link in your invitation email."
      />

      {error && <Banner variant={"danger" as never}>{error}</Banner>}

      {invites === null && !error && (
        <Empty
          title="Loading…"
          description="Fetching pending invitations from InvitationService.ListPendingInvitations."
        />
      )}

      {invites !== null && invites.length === 0 && (
        <Empty
          title="No pending invitations"
          description="When someone invites you to a billing account or organization, it shows up here."
        />
      )}

      {invites && invites.length > 0 && (
        <Table>
          <Table.Header>
            <Table.Row>
              <Table.Head>Scope</Table.Head>
              <Table.Head>Kind</Table.Head>
              <Table.Head>Role</Table.Head>
              <Table.Head>Status</Table.Head>
              <Table.Head>Expires</Table.Head>
            </Table.Row>
          </Table.Header>
          <Table.Body>
            {invites.map((inv) => (
              <Table.Row key={inv.id}>
                <Table.Cell>
                  <Text variant="body">{inv.scopeDisplayName || "—"}</Text>
                </Table.Cell>
                <Table.Cell>{SCOPE_LABEL[inv.scopeKind]}</Table.Cell>
                <Table.Cell>{ROLE_LABEL[inv.role]}</Table.Cell>
                <Table.Cell>{statusBadge(inv.status)}</Table.Cell>
                <Table.Cell>
                  <span className="text-kumo-subtle">{relativeExpiry(inv.expiresAt)}</span>
                </Table.Cell>
              </Table.Row>
            ))}
          </Table.Body>
        </Table>
      )}

      <Banner variant={"info" as never}>
        <Text variant="body">
          To accept or decline, follow the link from your invitation email.
          That flow lives at <code className="font-mono">/invite/:token</code>{" "}
          and verifies the macaroon. List-and-act in-app needs an
          <code className="font-mono"> AcceptInvitationById</code> RPC that
          isn't in the proto yet.
        </Text>
      </Banner>
    </div>
  );
}
