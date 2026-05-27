import { Badge, Banner, Breadcrumbs, Button, Text } from "@cloudflare/kumo";
import { PageHeader } from "../components/kumo/page-header/page-header";
import { ResourceListPage } from "../components/kumo/resource-list/resource-list";

export function Members() {
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
        description="Users with access to the current scope (billing account or organization)."
      />

      <Banner variant="default">
        <Text variant="body">
          Backend wiring lands with ROADMAP item 4
          (<code className="font-mono">whoami:permissions</code>) and the
          Cedar middleware. Until then this page is a layout placeholder.
        </Text>
      </Banner>

      <ResourceListPage
        title="People"
        description="Members of this scope, with their roles and last-seen times."
      >
        <div className="py-12 text-center text-kumo-subtle">
          <Text variant="body">
            No members loaded. Members are owners or members of the active
            billing/org scope; the data comes from{" "}
            <code className="font-mono">OrgService.ListMembers</code> /{" "}
            <code className="font-mono">BillingService.ListMembers</code>.
          </Text>
          <div className="mt-6 flex justify-center gap-3">
            <Button>Invite member</Button>
            <Button variant="secondary">
              <Badge variant="neutral">soon</Badge>
            </Button>
          </div>
        </div>
      </ResourceListPage>
    </div>
  );
}
