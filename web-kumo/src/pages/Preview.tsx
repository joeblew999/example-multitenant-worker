// A no-auth sandbox page that demos the Kumo AppShell + Orange theme.
// Used during development to visually verify the floor without standing
// up the worker. Remove or guard for dev-only once the real pages migrate.

import { Badge, Breadcrumbs, Button, LayerCard, Text } from "@cloudflare/kumo";
import { PageHeader } from "../components/kumo/page-header/page-header";

export function Preview() {
  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        breadcrumbs={
          <Breadcrumbs>
            <Breadcrumbs.Link href="/">Home</Breadcrumbs.Link>
            <Breadcrumbs.Separator />
            <Breadcrumbs.Current>Preview</Breadcrumbs.Current>
          </Breadcrumbs>
        }
        title="Kumo + Cloudflare Orange"
        description="Phase 1 floor — Tailwind v4, Kumo styles, Orange brand mark. PageHeader source lives at src/components/kumo/page-header/ thanks to `kumo add`."
      />

      <div className="flex items-center gap-3">
        <Badge variant="orange">Phase 1</Badge>
        <Badge variant="info">cedar branch</Badge>
        <Badge variant="neutral">3 blocks installed</Badge>
      </div>

      <LayerCard className="p-6">
        <div className="flex flex-col gap-4">
          <Text as="h2" variant="heading2">Brand color check</Text>
          <div className="flex items-center gap-3">
            <span className="size-8 rounded bg-kumo-brand" />
            <span className="font-mono text-kumo-subtle">
              bg-kumo-brand · #f6821f
            </span>
          </div>
          <div className="flex gap-3">
            <Button>Primary action</Button>
            <Button variant="secondary">Secondary</Button>
            <Button variant="ghost">Ghost</Button>
            <Button variant="destructive">Destructive</Button>
          </div>
        </div>
      </LayerCard>

      <LayerCard className="p-6">
        <div className="mb-3">
          <Text as="h2" variant="heading2">Blocks installed via `kumo add`</Text>
        </div>
        <ul className="list-disc space-y-1 pl-5 text-kumo-default">
          <li>
            <code className="font-mono text-kumo-subtle">PageHeader</code> —
            in use above (this title bar)
          </li>
          <li>
            <code className="font-mono text-kumo-subtle">ResourceListPage</code> —
            ready for org / member / billing list pages
          </li>
          <li>
            <code className="font-mono text-kumo-subtle">DeleteResource</code> —
            ready for destructive confirmation flows
          </li>
        </ul>
      </LayerCard>
    </div>
  );
}
