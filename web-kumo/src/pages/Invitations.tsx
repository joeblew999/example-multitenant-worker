import { Badge, Banner, Breadcrumbs, Button, Text } from "@cloudflare/kumo";
import { PageHeader } from "../components/kumo/page-header/page-header";
import { ResourceListPage } from "../components/kumo/resource-list/resource-list";

export function Invitations() {
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
        description="Pending invites where you're the invitee, and invites you've sent to others."
      />

      <Banner variant="default">
        <Text variant="body">
          Backend wiring is the{" "}
          <code className="font-mono">InvitationService</code> RPCs. The
          accept/decline flow at <code className="font-mono">/invite/:token</code>{" "}
          intentionally keeps the legacy auth aesthetic — it's an entry-point
          page, not an in-app one.
        </Text>
      </Banner>

      <ResourceListPage
        title="Pending invitations"
        description="Invites waiting for you to accept or decline."
      >
        <div className="py-12 text-center text-kumo-subtle">
          <Text variant="body">
            No pending invitations. Backed by{" "}
            <code className="font-mono">
              InvitationService.ListPendingInvitations
            </code>
            .
          </Text>
        </div>
      </ResourceListPage>

      <ResourceListPage
        title="Sent invitations"
        description="Invites you've sent on behalf of the current org. Owner-only."
      >
        <div className="py-12 text-center text-kumo-subtle">
          <Text variant="body">
            No sent invitations. Owner-only — gated by Cedar{" "}
            <code className="font-mono">OrgOwnerActions</code>.
          </Text>
          <div className="mt-6 flex justify-center gap-3">
            <Button>Send invitation</Button>
            <Button variant="secondary">
              <Badge variant="neutral">soon</Badge>
            </Button>
          </div>
        </div>
      </ResourceListPage>
    </div>
  );
}
