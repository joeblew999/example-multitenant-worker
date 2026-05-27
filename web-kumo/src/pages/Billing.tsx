import { Badge, Banner, Breadcrumbs, Button, LayerCard, Text } from "@cloudflare/kumo";
import { PageHeader } from "../components/kumo/page-header/page-header";

export function Billing() {
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
        description="Subscription, payment method, and invoices for the current billing account."
      />

      <Banner variant="default">
        <Text variant="body">
          Backend wiring lands when Cedar middleware enforces{" "}
          <code className="font-mono">BillingOwnerActions</code>. Until then
          this page is a layout placeholder.
        </Text>
      </Banner>

      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        <LayerCard className="p-6">
          <div className="mb-3 flex items-center justify-between">
            <Text as="h2" variant="heading2">Subscription</Text>
            <Badge variant="orange">active</Badge>
          </div>
          <Text variant="secondary">
            Plan, renewal date, status. Powered by{" "}
            <code className="font-mono">BillingService.GetSubscription</code>.
          </Text>
          <div className="mt-4 flex gap-3">
            <Button>Manage</Button>
            <Button variant="secondary">Change plan</Button>
          </div>
        </LayerCard>

        <LayerCard className="p-6">
          <Text as="h2" variant="heading2">Payment method</Text>
          <div className="mt-2">
            <Text variant="secondary">
              Card on file. Updated via{" "}
              <code className="font-mono">BillingService.SetPaymentMethod</code>
              {" "}— Cedar requires{" "}
              <code className="font-mono">role == owner</code> in the active
              billing scope.
            </Text>
          </div>
          <div className="mt-4">
            <Button variant="secondary">Update payment method</Button>
          </div>
        </LayerCard>
      </div>

      <LayerCard className="p-6">
        <div className="mb-3 flex items-center justify-between">
          <Text as="h2" variant="heading2">Invoices</Text>
          <Button variant="ghost">Download all</Button>
        </div>
        <div className="py-8 text-center text-kumo-subtle">
          <Text variant="body">
            No invoices loaded. Comes from{" "}
            <code className="font-mono">BillingService.ListInvoices</code>.
          </Text>
        </div>
      </LayerCard>
    </div>
  );
}
