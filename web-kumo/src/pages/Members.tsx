import {
  Badge,
  Banner,
  Breadcrumbs,
  Empty,
  Table,
  Text,
} from "@cloudflare/kumo";
import { PageHeader } from "../components/kumo/page-header/page-header";
import { useAuth } from "../auth";
import { Role } from "../../gen/workers/auth/v1/auth_pb.js";

function roleBadge(role: Role) {
  const isOwner = role === Role.OWNER;
  return <Badge variant={isOwner ? "orange" : "neutral"}>{isOwner ? "owner" : "member"}</Badge>;
}

export function Members() {
  const { state } = useAuth();
  if (state.status !== "authenticated") return null;
  const { whoami } = state;

  type Row = {
    kind: "Billing" | "Org";
    scopeId: string;
    displayName: string;
    role: Role;
  };
  const rows: Row[] = [
    ...whoami.billingMemberships.map((m) => ({
      kind: "Billing" as const,
      scopeId: m.scopeId,
      displayName: m.displayName,
      role: m.role,
    })),
    ...whoami.orgMemberships.map((m) => ({
      kind: "Org" as const,
      scopeId: m.scopeId,
      displayName: m.displayName,
      role: m.role,
    })),
  ];

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        breadcrumbs={
          <Breadcrumbs>
            <Breadcrumbs.Link href="/">Home</Breadcrumbs.Link>
            <Breadcrumbs.Separator />
            <Breadcrumbs.Current>Members</Breadcrumbs.Current>
          </Breadcrumbs>
        }
        title="Members"
        description="Your memberships across billing and org scopes. The full per-scope member list lands with Cedar + whoami:permissions (ROADMAP item 4)."
      />

      <Banner variant="default">
        <Text variant="body">
          Showing data from the <code className="font-mono">whoami</code>{" "}
          session — your own scope memberships. There isn't yet a{" "}
          <code className="font-mono">ListMembers</code> RPC to enumerate
          others in each scope; that's gated behind the Cedar middleware
          rollout.
        </Text>
      </Banner>

      {rows.length === 0 ? (
        <Empty title="No memberships" description="You don't belong to any billing or org scope." />
      ) : (
        <Table>
          <Table.Header>
            <Table.Row>
              <Table.Head>Scope</Table.Head>
              <Table.Head>Kind</Table.Head>
              <Table.Head>Your role</Table.Head>
              <Table.Head>ID</Table.Head>
            </Table.Row>
          </Table.Header>
          <Table.Body>
            {rows.map((r) => (
              <Table.Row key={`${r.kind}-${r.scopeId}`}>
                <Table.Cell>{r.displayName}</Table.Cell>
                <Table.Cell>
                  <Badge variant={r.kind === "Billing" ? "blue" : "purple"}>
                    {r.kind}
                  </Badge>
                </Table.Cell>
                <Table.Cell>{roleBadge(r.role)}</Table.Cell>
                <Table.Cell>
                  <code className="text-xs font-mono text-kumo-subtle">{r.scopeId}</code>
                </Table.Cell>
              </Table.Row>
            ))}
          </Table.Body>
        </Table>
      )}
    </div>
  );
}
