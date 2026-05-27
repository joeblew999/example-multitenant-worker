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
import { billingClient, errorMessage } from "../client";
import { useAuth } from "../auth";
import type { BillingAccount, Subscription } from "../../gen/workers/billing/v1/billing_pb.js";
import { SubscriptionStatus } from "../../gen/workers/billing/v1/billing_pb.js";

function statusBadge(s: SubscriptionStatus) {
  switch (s) {
    case SubscriptionStatus.ACTIVE:
      return <Badge variant={"green" as never}>active</Badge>;
    case SubscriptionStatus.PAST_DUE:
      return <Badge variant={"orange" as never}>past due</Badge>;
    case SubscriptionStatus.CANCELED:
      return <Badge variant={"red" as never}>canceled</Badge>;
    case SubscriptionStatus.NONE:
      return <Badge variant={"neutral" as never}>none</Badge>;
    default:
      return <Badge variant={"neutral" as never}>unknown</Badge>;
  }
}

export function Billing() {
  const { state } = useAuth();
  const [accounts, setAccounts] = useState<BillingAccount[] | null>(null);
  const [sub, setSub] = useState<Subscription | null>(null);
  const [error, setError] = useState<string | null>(null);

  const billingAccountId =
    state.status === "authenticated" ? state.whoami.billingAccountId : "";

  useEffect(() => {
    if (state.status !== "authenticated") return;
    let cancelled = false;
    Promise.all([
      billingClient.listBillingAccounts({}),
      billingClient.getSubscription({ billingAccountId }),
    ])
      .then(([list, subRes]) => {
        if (cancelled) return;
        setAccounts(list.accounts);
        setSub(subRes.subscription ?? null);
      })
      .catch((err) => {
        if (!cancelled) setError(errorMessage(err, "failed to load billing"));
      });
    return () => {
      cancelled = true;
    };
  }, [state.status, billingAccountId]);

  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        breadcrumbs={
          <Breadcrumbs>
            <Breadcrumbs.Link href="/">Home</Breadcrumbs.Link>
            <Breadcrumbs.Separator />
            <Breadcrumbs.Current>Billing</Breadcrumbs.Current>
          </Breadcrumbs>
        }
        title="Billing"
        description="Billing accounts you own or belong to. Each personal sign-up mints one; orgs nest beneath them."
      />

      {error && <Banner variant={"danger" as never}>{error}</Banner>}

      {accounts === null && !error && (
        <Empty title="Loading…" description="Fetching from BillingService.ListBillingAccounts." />
      )}

      {sub && (
        <div className="flex flex-col gap-3 p-6 rounded-lg bg-kumo-base ring ring-kumo-line">
          <Text variant="heading2">Current subscription</Text>
          <div className="flex items-center gap-4 flex-wrap">
            <Text variant="body" className="text-kumo-subtle">Plan</Text>
            <Badge variant={"neutral" as never}>{sub.plan || "—"}</Badge>
            <Text variant="body" className="text-kumo-subtle">Status</Text>
            {statusBadge(sub.status)}
          </div>
        </div>
      )}

      {accounts && accounts.length === 0 && (
        <Empty title="No billing accounts" description="You don't belong to any billing scope yet." />
      )}

      {accounts && accounts.length > 0 && (
        <Table>
          <Table.Header>
            <Table.Row>
              <Table.Head>Name</Table.Head>
              <Table.Head>Kind</Table.Head>
              <Table.Head>Auto-join domain</Table.Head>
              <Table.Head>SSO</Table.Head>
            </Table.Row>
          </Table.Header>
          <Table.Body>
            {accounts.map((a) => (
              <Table.Row key={a.id}>
                <Table.Cell>
                  <div className="flex flex-col">
                    <Text variant="body">{a.displayName}</Text>
                    <code className="text-xs font-mono text-kumo-subtle">{a.id}</code>
                  </div>
                </Table.Cell>
                <Table.Cell>
                  <Badge variant={(a.personal ? "neutral" : "blue") as never}>
                    {a.personal ? "personal" : "shared"}
                  </Badge>
                </Table.Cell>
                <Table.Cell>
                  <span className="text-kumo-subtle">{a.autoJoinDomain || "—"}</span>
                </Table.Cell>
                <Table.Cell>
                  {a.sso ? (
                    <Badge variant={"green" as never}>configured</Badge>
                  ) : (
                    <span className="text-kumo-subtle">—</span>
                  )}
                </Table.Cell>
              </Table.Row>
            ))}
          </Table.Body>
        </Table>
      )}
    </div>
  );
}
