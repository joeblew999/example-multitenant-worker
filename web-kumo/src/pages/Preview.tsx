// Theme + component showcase. Lives at /preview.
//
// /preview = developer debug page. Testers click DemoAccounts, flip
// themes, see how every Kumo primitive looks under each theme.

import { useState } from "react";
import {
  Badge,
  Banner,
  Breadcrumbs,
  Button,
  Checkbox,
  ClipboardText,
  CodeBlock,
  Collapsible,
  Empty,
  Field,
  Input,
  InputArea,
  InputGroup,
  LayerCard,
  Link,
  LinkButton,
  Loader,
  Meter,
  Pagination,
  Radio,
  RadioGroup,
  Select,
  SensitiveInput,
  SkeletonLine,
  Surface,
  Switch,
  Table,
  Tabs,
  Text,
  Textarea,
} from "@cloudflare/kumo";
import { PageHeader } from "../components/kumo/page-header/page-header";
import { DevAccounts } from "../components/DevAccounts";
import { ThemeToggle } from "../components/ThemeToggle";

// Kumo Badge has both color-only variants (red/green/orange/etc.) and
// semantic variants (primary/secondary/success/etc.). Showcase the
// color ones — semantic ones are covered implicitly by status badges
// in the Table section.
const BADGE_COLORS = [
  "orange", "red", "purple", "green", "teal", "blue", "neutral", "info",
] as const;

// Real Kumo Banner variants — only three. info/warning/danger/success
// silently fall back to default. The intent-color tints they used to
// imply now come from the Toast/Badge/Banner system more directly.
const BANNER_VARIANTS = ["default", "alert", "error"] as const;

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
  const [secret, setSecret] = useState("");
  const [meterValue, setMeterValue] = useState(64);

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
        title="Theme & component showcase"
        description="Every Kumo primitive we use, in one place. Flip themes via the card below and watch what breaks."
      />

      <LayerCard className="p-6">
        <div className="mb-3">
          <Text as="h2" variant="heading2">Theme</Text>
          <p className="mt-1 text-kumo-subtle text-sm">
            Live A/B switch between the editorial theme and Kumo's defaults.
            Persists per-browser in <code className="font-mono">localStorage["wm.theme"]</code>.
          </p>
        </div>
        <ThemeToggle />
      </LayerCard>

      <DevAccounts />

      {/* ── Typography ── */}
      <Section title="Typography" hint="Text variants — heading scale, body, code, monospace.">
        <Text as="h3" variant="heading1">Heading 1 — Display</Text>
        <Text as="h3" variant="heading2">Heading 2 — Section</Text>
        <Text as="h3" variant="heading3">Heading 3 — Subsection</Text>
        <Text variant="body">Body text. The quick brown fox jumps over the lazy dog.</Text>
        <Text variant="secondary">Secondary text — annotation, less emphasis.</Text>
        <Text variant="mono">Mono variant for terminal-style content.</Text>
        <p>Inline <code className="font-mono">code</code> renders inside paragraphs.</p>
        <div>Block via Kumo's CodeBlock:</div>
        <CodeBlock code={`// Block code\nconst sum = (a, b) => a + b;`} />
      </Section>

      {/* ── Buttons ── */}
      <Section title="Buttons" hint="Variants + sizes.">
        <div className="flex flex-wrap gap-3 items-center">
          <Button variant="primary">Primary</Button>
          <Button variant="secondary">Secondary</Button>
          <Button variant="ghost">Ghost</Button>
          <Button variant="destructive">Destructive</Button>
          <Button variant="secondary" disabled>Disabled</Button>
        </div>
        <div className="flex flex-wrap gap-3 items-center">
          <Button variant="primary" size="xs">XS</Button>
          <Button variant="primary" size="sm">SM</Button>
          <Button variant="primary" size="base">Base</Button>
          <Button variant="primary" size="lg">LG</Button>
        </div>
        <div className="flex gap-3 items-center">
          <LinkButton href="/" variant="secondary">LinkButton (anchor styled as button)</LinkButton>
        </div>
      </Section>

      {/* ── Badges ── */}
      <Section title="Badges" hint="Color set + status pills.">
        <div className="flex flex-wrap gap-2">
          {BADGE_COLORS.map((c) => (
            <Badge key={c} variant={c}>{c}</Badge>
          ))}
        </div>
      </Section>

      {/* ── Banners ── */}
      <Section title="Banners" hint="Intent variants — each should read distinctly.">
        {BANNER_VARIANTS.map((v) => (
          <Banner key={v} variant={v}>
            <Text variant="body">
              <b>{v}</b> — the quick brown fox jumps over the lazy dog.
            </Text>
          </Banner>
        ))}
      </Section>

      {/* ── Form controls ── */}
      <Section title="Form controls" hint="Field + Input + Textarea + Select + Checkbox + Switch + Radio.">
        <Field label="Email">
          <Input type="email" placeholder="you@example.com" />
        </Field>
        <Field label="Sensitive input (masked)">
          <SensitiveInput value={secret} onValueChange={setSecret} placeholder="api-key-..." />
        </Field>
        <Field label="Notes">
          <Textarea placeholder="Multi-line input…" rows={3} />
        </Field>
        <Field label="Long-form (InputArea — auto-grow)">
          <InputArea placeholder="Auto-growing area..." />
        </Field>
        <Field label="Region (Select)">
          <Select
            value={select}
            onValueChange={(v) => setSelect(String(v))}
            items={{ "us-east": "US East", "eu-west": "EU West", "ap-south": "AP South" }}
          />
        </Field>
        <div className="flex items-center gap-6">
          <Field label="Send me updates" controlFirst>
            <Checkbox checked={check} onCheckedChange={(v) => setCheck(Boolean(v))} />
          </Field>
          <Field label="Notifications" controlFirst>
            <Switch checked={sw} onCheckedChange={setSw} />
          </Field>
        </div>
        <RadioGroup
          legend="Option"
          value={radio}
          onValueChange={(v) => setRadio(String(v))}
        >
          <Radio.Item label="Option one" value="one" />
          <Radio.Item label="Option two" value="two" />
          <Radio.Item label="Option three" value="three" />
        </RadioGroup>
      </Section>

      {/* ── Advanced inputs ── */}
      <Section title="Advanced inputs" hint="InputGroup (addon + suffix), ClipboardText, Meter.">
        <Field label="URL with prefix">
          <InputGroup>
            <InputGroup.Addon>https://</InputGroup.Addon>
            <InputGroup.Input placeholder="example" />
            <InputGroup.Suffix>.workers.dev</InputGroup.Suffix>
          </InputGroup>
        </Field>
        <Field label="Copyable token">
          <ClipboardText text="ed_42a8c1f9b3e4d2a7c5e1f8b9d3c4e5a6b7c8d9e1" />
        </Field>
        <Field label="Quota">
          <Meter value={meterValue} max={100} label={`Quota (${meterValue}%)`} />
          <input
            type="range" min={0} max={100} value={meterValue}
            onChange={(e) => setMeterValue(Number(e.target.value))}
            className="mt-2 w-full"
          />
        </Field>
      </Section>

      {/* ── Collapsible ── */}
      <Section title="Collapsible" hint="Disclosure widget — toggleable content panel.">
        <Collapsible defaultOpen>
          <Collapsible.Trigger className="font-medium hover:underline">
            Click to toggle ▾
          </Collapsible.Trigger>
          <Collapsible.Panel>
            <p className="mt-2 text-kumo-subtle">
              Hidden content revealed by the trigger.
            </p>
          </Collapsible.Panel>
        </Collapsible>
      </Section>

      {/* ── Tabs ── */}
      <Section title="Tabs" hint="Tab list + content area. Active indicator uses --color-kumo-brand.">
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

      {/* ── Table + Pagination ── */}
      <Section title="Table" hint="Header row + body rows + Badge in cell + Pagination.">
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
              <Table.Cell><Badge variant="green">active</Badge></Table.Cell>
            </Table.Row>
            <Table.Row>
              <Table.Cell>alice@acme.example</Table.Cell>
              <Table.Cell>member</Table.Cell>
              <Table.Cell><Badge variant="neutral">invited</Badge></Table.Cell>
            </Table.Row>
            <Table.Row>
              <Table.Cell>bob@partner.example</Table.Cell>
              <Table.Cell>member</Table.Cell>
              <Table.Cell><Badge variant="red">revoked</Badge></Table.Cell>
            </Table.Row>
          </Table.Body>
        </Table>
        <Pagination page={2} setPage={() => {}} perPage={10} totalCount={42} />
      </Section>

      {/* ── Links ── */}
      <Section title="Links" hint="Link (inline) + LinkButton (button-shaped).">
        <div className="flex flex-wrap gap-4 items-center">
          <Link href="/">Inline link</Link>
          <LinkButton href="/login" variant="primary">Primary link-button</LinkButton>
          <LinkButton href="/login" variant="secondary">Secondary link-button</LinkButton>
        </div>
      </Section>

      {/* ── Surfaces ── */}
      <Section title="Surface" hint="Themed bg+border containers (color: primary | secondary). Surface is a deprecated wrapper around LayerCard — kept here for completeness.">
        <div className="grid grid-cols-2 gap-3">
          {(["primary", "secondary"] as const).map((c) => (
            <Surface key={c} color={c} className="p-4 rounded-lg ring ring-kumo-line">
              <Text variant="body">color="{c}"</Text>
            </Surface>
          ))}
        </div>
      </Section>

      {/* ── Loading + empty ── */}
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

      {/* ── Not yet on this page ── */}
      <Section title="Not yet on this page" hint="Components with more complex APIs that need a dedicated demo pass.">
        <ul className="list-disc pl-5 text-kumo-subtle text-sm space-y-1">
          <li>
            <code className="font-mono">Dialog</code> / <code className="font-mono">Popover</code> / <code className="font-mono">DropdownMenu</code> /
            <code className="font-mono">Tooltip</code> — Base UI compound primitives; need
            <code className="font-mono">{"render={...}"}</code> pattern + portal positioning. Have
            <code className="font-mono">KumoPortalProvider</code> mounted in main.tsx ready.
          </li>
          <li>
            <code className="font-mono">Combobox</code> / <code className="font-mono">Autocomplete</code> — full picker APIs
            (multi-select, async loading) deserve their own section.
          </li>
          <li>
            <code className="font-mono">DatePicker</code> / <code className="font-mono">DateRangePicker</code> — date primitives.
          </li>
          <li>
            <code className="font-mono">Chart</code> / <code className="font-mono">TimeseriesChart</code> / <code className="font-mono">SankeyChart</code> —
            need real data series.
          </li>
          <li>
            <code className="font-mono">CommandPalette</code> / <code className="font-mono">MenuBar</code> / <code className="font-mono">TableOfContents</code> /
            <code className="font-mono">Toast</code> — overlay + nav primitives.
          </li>
        </ul>
      </Section>
    </div>
  );
}
