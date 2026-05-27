import { useState } from "react";
import type { MembershipSummary } from "../../gen/workers/auth/v1/auth_pb.js";
import { AuthMethodKind, Role } from "../../gen/workers/auth/v1/auth_pb.js";
import {
  Badge,
  Banner,
  Breadcrumbs,
  Button,
  LayerCard,
  Table,
  Text,
} from "@cloudflare/kumo";
import { PageHeader } from "../components/kumo/page-header/page-header";
import { useAuth } from "../auth";
import { authClient, errorMessage } from "../client";

const roleLabel: Record<Role, string> = {
  [Role.OWNER]: "owner",
  [Role.MEMBER]: "member",
  [Role.UNSPECIFIED]: "unknown",
};

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

  const authMethodLabel =
    whoami.authMethod?.kind === AuthMethodKind.SSO
      ? `sso${whoami.authMethod.idpId ? `:${whoami.authMethod.idpId}` : ""}`
      : "password";

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        breadcrumbs={
          <Breadcrumbs>
            <Breadcrumbs.Link href="/">Home</Breadcrumbs.Link>
            <Breadcrumbs.Separator />
            <Breadcrumbs.Current>Dashboard</Breadcrumbs.Current>
          </Breadcrumbs>
        }
        title={currentScopeName}
        description={`${currentScopeKind} · Operating as ${whoami.email} with role ${roleLabel[whoami.role]}. Switch scopes below — every pivot mints a fresh, narrowly-scoped session token.`}
      />

      {error && (
        <Banner variant="error">
          <span>{error}</span>
        </Banner>
      )}

      <LayerCard className="p-6">
        <div className="flex items-center justify-between mb-4">
          <Text as="h2" variant="heading2">Account</Text>
          <Button
            variant="ghost"
            onClick={() => {
              refreshWhoami().catch((e) => setError(errorMessage(e, "refresh failed")));
            }}
          >
            Refresh
          </Button>
        </div>

        <KvList
          rows={[
            { k: "User ID", v: <span className="font-mono text-kumo-default">{whoami.userId}</span> },
            {
              k: "Email",
              v: (
                <span>
                  {whoami.email}
                  {!whoami.emailVerified && (
                    <span className="ml-2 text-kumo-subtle">· unverified</span>
                  )}
                </span>
              ),
            },
            { k: "Auth method", v: <Badge variant="neutral">{authMethodLabel}</Badge> },
            { k: "Active role", v: <Badge variant="orange">{roleLabel[whoami.role]}</Badge> },
            {
              k: "Billing scope",
              v: <span className="font-mono text-kumo-default">{whoami.billingAccountId}</span>,
            },
            {
              k: "Org scope",
              v: whoami.orgId ? (
                <span className="font-mono text-kumo-default">{whoami.orgId}</span>
              ) : (
                <span className="text-kumo-subtle">— operating at billing scope</span>
              ),
            },
          ]}
        />
      </LayerCard>

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

function KvList({ rows }: { rows: { k: string; v: React.ReactNode }[] }) {
  return (
    <dl className="grid grid-cols-[minmax(140px,200px)_1fr] gap-x-6">
      {rows.map(({ k, v }, i) => (
        <div key={k} className={`contents`}>
          <dt
            className={`py-3 text-sm uppercase tracking-wider text-kumo-subtle ${
              i > 0 ? "border-t border-kumo-line" : ""
            }`}
          >
            {k}
          </dt>
          <dd
            className={`py-3 text-kumo-default ${
              i > 0 ? "border-t border-kumo-line" : ""
            }`}
          >
            {v}
          </dd>
        </div>
      ))}
    </dl>
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
    <LayerCard className="p-6">
      <div className="flex items-baseline justify-between mb-1">
        <Text as="h2" variant="heading2">{title}</Text>
        <span className="font-mono text-sm text-kumo-subtle">
          {items.length.toString().padStart(2, "0")} ·{" "}
          {items.length === 1 ? "membership" : "memberships"}
        </span>
      </div>
      <Text variant="secondary">{subtitle}</Text>

      <div className="mt-4">
        {items.length === 0 ? (
          <div className="py-8 text-center text-kumo-subtle">No memberships</div>
        ) : (
          <Table>
            <Table.Header>
              <Table.Row>
                <Table.Head>Name</Table.Head>
                <Table.Head>Role</Table.Head>
                <Table.Head>ID</Table.Head>
                <Table.Head>Status</Table.Head>
              </Table.Row>
            </Table.Header>
            <Table.Body>
              {items.map((m) => {
                const k = `${keyPrefix}:${m.scopeId}`;
                const current = m.scopeId === currentScopeId;
                return (
                  <Table.Row key={m.scopeId} variant={current ? "selected" : "default"}>
                    <Table.Cell>
                      <span className="font-medium text-kumo-strong">
                        {m.displayName || m.scopeId}
                      </span>
                    </Table.Cell>
                    <Table.Cell>
                      <Badge variant={m.role === Role.OWNER ? "orange" : "neutral"}>
                        {roleLabel[m.role]}
                      </Badge>
                    </Table.Cell>
                    <Table.Cell>
                      <span className="font-mono text-xs text-kumo-subtle">{m.scopeId}</span>
                    </Table.Cell>
                    <Table.Cell>
                      {current ? (
                        <Badge variant="success">Active</Badge>
                      ) : (
                        <Button
                          variant="secondary"
                          disabled={busyId !== null}
                          onClick={() => onSelect(m.scopeId)}
                        >
                          {busyId === k ? "Switching…" : "Switch"}
                        </Button>
                      )}
                    </Table.Cell>
                  </Table.Row>
                );
              })}
            </Table.Body>
          </Table>
        )}
      </div>
    </LayerCard>
  );
}
