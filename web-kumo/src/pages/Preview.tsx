// A no-auth sandbox page that demos the Kumo AppShell + Orange theme.
// Used during development to visually verify the floor without standing
// up the worker. Remove or guard for dev-only once the real pages migrate.

import { Badge, Button, LayerCard, Text } from "@cloudflare/kumo";

export function Preview() {
  return (
    <div className="flex flex-col gap-6">
      <header className="flex items-center justify-between">
        <div className="flex flex-col gap-1">
          <Text as="h1" variant="heading1">Kumo + Cloudflare Orange</Text>
          <Text variant="secondary">
            Phase 1 floor — Tailwind v4, Kumo styles, Orange brand mark.
          </Text>
        </div>
        <div className="flex items-center gap-3">
          <Badge variant="orange">Phase 1</Badge>
          <Badge variant="info">cedar branch</Badge>
        </div>
      </header>

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
          </div>
        </div>
      </LayerCard>

      <LayerCard className="p-6">
        <div className="mb-3">
          <Text as="h2" variant="heading2">What's next</Text>
        </div>
        <ul className="list-disc space-y-1 pl-5 text-kumo-default">
          <li>Phase 2: convert auth pages (Login, Signup, AcceptInvite)</li>
          <li>Phase 3-4: convert org / billing / permissions pages</li>
          <li>Phase 5: convert invitation flows</li>
        </ul>
      </LayerCard>
    </div>
  );
}
