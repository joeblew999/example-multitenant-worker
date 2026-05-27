// Component showcase for theme stress-testing.
//
// Renders every Kumo primitive we expect to use on real pages, so we can
// flip themes via the ThemeToggle and visually audit token coverage in
// one place. If a page later adds a component that's not on this list,
// add it here too — the showcase is the source of truth for "what the
// editorial theme must handle."

import { useState } from "react";
import {
  Badge,
  Banner,
  Breadcrumbs,
  Button,
  Checkbox,
  Empty,
  Field,
  Input,
  Label,
  LayerCard,
  Loader,
  Pagination,
  Radio,
  RadioGroup,
  Select,
  SkeletonLine,
  Switch,
  Table,
  Tabs,
  Text,
  Textarea,
} from "@cloudflare/kumo";
import { PageHeader } from "../components/kumo/page-header/page-header";

const BADGE_COLORS = [
  "orange",
  "red",
  "purple",
  "green",
  "teal",
  "blue",
  "neutral",
  "info",
] as const;

const BANNER_VARIANTS = ["default", "info", "warning", "danger", "success"] as const;

function Section({ title, hint, children }: { title: string; hint?: string; children: React.ReactNode }) {
  return (
    <LayerCard className="p-6">
      <div className="mb-4">
        <Text as="h2" variant="heading2">{title}</Text>
        {hint && <p className="mt-1 text-kumo-subtle text-sm">{hint}</p>}
      </div>
      <div className="flex flex-col gap-4">{children}</div>
    </LayerCard>
  );
}

export function Preview() {
  const [tab, setTab] = useState("alpha");
  const [radio, setRadio] = useState("two");
  const [select, setSelect] = useState("eu-west");
  const [check, setCheck] = useState(true);
  const [sw, setSw] = useState(true);

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
          title="Theme showcase"
          description="Every Kumo primitive we use on real pages, in one place. Flip themes via the toggle in the top-right and watch what breaks."
        />

        <Section title="Buttons" hint="All variants. Primary uses --color-kumo-brand; destructive uses --color-kumo-danger.">
          <div className="flex flex-wrap gap-3">
            <Button>Primary</Button>
            <Button variant="secondary">Secondary</Button>
            <Button variant="ghost">Ghost</Button>
            <Button variant="destructive">Destructive</Button>
            <Button disabled>Disabled</Button>
          </div>
        </Section>

        <Section title="Badges" hint="Filled (top) and subtle (bottom). Subtle variants are tints; useful for status pills.">
          <div className="flex flex-wrap gap-2">
            {BADGE_COLORS.map((c) => (
              <Badge key={c} variant={c as never}>{c}</Badge>
            ))}
          </div>
        </Section>

        <Section title="Banners" hint="Five intent variants — each should read clearly without surface bleed.">
          {BANNER_VARIANTS.map((v) => (
            <Banner key={v} variant={v as never}>
              <Text variant="body">
                <b>{v}</b> — the quick brown fox jumps over the lazy dog.
              </Text>
            </Banner>
          ))}
        </Section>

        <Section title="Form controls" hint="Field + Label + Input + Textarea + Select + Checkbox + Radio + Switch.">
          <Field>
            <Label>Email</Label>
            <Input type="email" placeholder="you@example.com" />
          </Field>
          <Field>
            <Label>Notes</Label>
            <Textarea placeholder="Multi-line input…" rows={3} />
          </Field>
          <Field>
            <Label>Region</Label>
            <Select value={select} onValueChange={(v) => setSelect(String(v))}>
              <Select.Trigger />
              <Select.Content>
                <Select.Item value="us-east">US East</Select.Item>
                <Select.Item value="eu-west">EU West</Select.Item>
                <Select.Item value="ap-south">AP South</Select.Item>
              </Select.Content>
            </Select>
          </Field>
          <div className="flex items-center gap-4">
            <Checkbox checked={check} onCheckedChange={(v) => setCheck(Boolean(v))}>
              Send me updates
            </Checkbox>
            <Switch checked={sw} onCheckedChange={setSw}>
              Notifications
            </Switch>
          </div>
          <RadioGroup value={radio} onValueChange={(v) => setRadio(String(v))}>
            <Radio value="one">Option one</Radio>
            <Radio value="two">Option two</Radio>
            <Radio value="three">Option three</Radio>
          </RadioGroup>
        </Section>

        <Section title="Tabs" hint="Tab list + content area. Active indicator uses --color-kumo-brand or --color-kumo-line.">
          <Tabs
            selectedValue={tab}
            onValueChange={(v) => setTab(String(v))}
            tabs={[
              { value: "alpha", label: "Alpha" },
              { value: "beta", label: "Beta" },
              { value: "gamma", label: "Gamma" },
            ]}
          />
          <p className="text-kumo-subtle">Selected: {tab}</p>
        </Section>

        <Section title="Table" hint="Header row + body rows + Badge in cell.">
          <Table>
            <Table.Header>
              <Table.Row>
                <Table.Head>Name</Table.Head>
                <Table.Head>Role</Table.Head>
                <Table.Head>Status</Table.Head>
              </Table.Row>
            </Table.Header>
            <Table.Body>
              <Table.Row>
                <Table.Cell>demo@example.com</Table.Cell>
                <Table.Cell>owner</Table.Cell>
                <Table.Cell><Badge variant={"green" as never}>active</Badge></Table.Cell>
              </Table.Row>
              <Table.Row>
                <Table.Cell>alice@acme.io</Table.Cell>
                <Table.Cell>member</Table.Cell>
                <Table.Cell><Badge variant={"neutral" as never}>invited</Badge></Table.Cell>
              </Table.Row>
              <Table.Row>
                <Table.Cell>bob@partner.dev</Table.Cell>
                <Table.Cell>member</Table.Cell>
                <Table.Cell><Badge variant={"red" as never}>revoked</Badge></Table.Cell>
              </Table.Row>
            </Table.Body>
          </Table>
          <Pagination total={42} pageSize={10} currentPage={2} onPageChange={() => {}} />
        </Section>

        <Section title="Loading + empty states" hint="Loader, SkeletonLine, Empty.">
          <div className="flex items-center gap-6">
            <Loader />
            <div className="flex-1 flex flex-col gap-2">
              <SkeletonLine />
              <SkeletonLine className="w-3/4" />
              <SkeletonLine className="w-1/2" />
            </div>
          </div>
          <Empty
            title="No results"
            description="No matches found. Try a different filter or come back later."
          />
        </Section>
      </div>
  );
}
