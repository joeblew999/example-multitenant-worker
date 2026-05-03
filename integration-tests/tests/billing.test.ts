import { describe, it, expect } from "vitest";
import { Code } from "@connectrpc/connect";
import { authClient, billingClient, signupAndGetToken, uniqueEmail } from "./helpers";
import {
  InvoiceStatus,
  Role,
  SsoKind,
  SubscriptionStatus,
} from "../gen/workers/billing/v1/billing_pb.js";

async function signupOwner() {
  return signupAndGetToken(uniqueEmail("owner"));
}

describe("BillingService — personal billing constraints", () => {
  it("personal billing accounts are non-deletable", async () => {
    const { sessionToken, whoami } = await signupOwner();
    const billing = billingClient({ sessionToken });
    await expect(
      billing.deleteBillingAccount({ billingAccountId: whoami.billingAccountId }),
    ).rejects.toMatchObject({ code: Code.FailedPrecondition });
  });

  it("auto_join_domain is rejected on personal billing", async () => {
    const { sessionToken, whoami } = await signupOwner();
    const billing = billingClient({ sessionToken });
    await expect(
      billing.setAutoJoinDomain({
        billingAccountId: whoami.billingAccountId,
        domain: "acme.com",
      }),
    ).rejects.toMatchObject({ code: Code.FailedPrecondition });
  });
});

describe("BillingService — invite + accept (existing user)", () => {
  it("owner invites a new user, who signs up via the invite link and lands as a member", async () => {
    // Personal billing also accepts invites — deliberate simplification of the demo.
    const owner = await signupOwner();
    const billing = billingClient({ sessionToken: owner.sessionToken });
    const inviteeEmail = uniqueEmail("invitee");
    const invite = await billing.inviteMember({
      billingAccountId: owner.whoami.billingAccountId,
      email: inviteeEmail,
      role: Role.MEMBER,
    });
    expect(invite.invitationId).toBeTruthy();
    expect(invite.inviteTokenForDemo).toBeTruthy();

    const signup = await authClient().signup({
      email: inviteeEmail,
      password: "another-very-long-password",
      inviteToken: invite.inviteTokenForDemo,
    });
    expect(signup.sessionToken).toBeTruthy();
    const billingIds = signup.whoami!.billingMemberships.map((m) => m.scopeId);
    expect(billingIds).toContain(owner.whoami.billingAccountId);
  });

  it("re-inviting upserts the invitation and rotates the token (old one stops working)", async () => {
    const owner = await signupOwner();
    const billing = billingClient({ sessionToken: owner.sessionToken });
    const email = uniqueEmail("rotater");
    const first = await billing.inviteMember({
      billingAccountId: owner.whoami.billingAccountId,
      email,
      role: Role.MEMBER,
    });
    const second = await billing.inviteMember({
      billingAccountId: owner.whoami.billingAccountId,
      email,
      role: Role.OWNER,
    });
    expect(first.invitationId).toBe(second.invitationId);
    expect(first.inviteTokenForDemo).not.toBe(second.inviteTokenForDemo);

    await expect(
      authClient().signup({
        email,
        password: "another-very-long-password",
        inviteToken: first.inviteTokenForDemo,
      }),
    ).rejects.toMatchObject({ code: Code.FailedPrecondition });
  });

  it("rejects invitations whose email doesn't match the signup email", async () => {
    const owner = await signupOwner();
    const billing = billingClient({ sessionToken: owner.sessionToken });
    const inv = await billing.inviteMember({
      billingAccountId: owner.whoami.billingAccountId,
      email: uniqueEmail("intended"),
      role: Role.MEMBER,
    });
    await expect(
      authClient().signup({
        email: uniqueEmail("imposter"),
        password: "another-very-long-password",
        inviteToken: inv.inviteTokenForDemo,
      }),
    ).rejects.toMatchObject({ code: Code.PermissionDenied });
  });
});

describe("BillingService — subscription / invoices", () => {
  it("get / set / cancel subscription round-trips for the owner", async () => {
    const { sessionToken, whoami } = await signupOwner();
    const billing = billingClient({ sessionToken });

    const initial = await billing.getSubscription({
      billingAccountId: whoami.billingAccountId,
    });
    expect(initial.subscription?.status).toBe(SubscriptionStatus.NONE);

    await billing.setPaymentMethod({
      billingAccountId: whoami.billingAccountId,
      paymentMethodToken: "tok_test_visa",
    });

    const active = await billing.getSubscription({
      billingAccountId: whoami.billingAccountId,
    });
    expect(active.subscription?.status).toBe(SubscriptionStatus.ACTIVE);
    expect(active.subscription?.plan).toBe("starter");
    expect(active.subscription?.paymentMethodToken).toBe("tok_test_visa");

    const canceled = await billing.cancelSubscription({
      billingAccountId: whoami.billingAccountId,
    });
    expect(canceled.subscription?.status).toBe(SubscriptionStatus.CANCELED);
  });

  it("non-members cannot read other accounts' subscriptions", async () => {
    const ownerA = await signupOwner();
    const ownerB = await signupOwner();
    const billingB = billingClient({ sessionToken: ownerB.sessionToken });
    await expect(
      billingB.getSubscription({ billingAccountId: ownerA.whoami.billingAccountId }),
    ).rejects.toMatchObject({ code: Code.PermissionDenied });
  });
});

describe("BillingService — listInvoices", () => {
  it("returns an empty invoice list for a fresh account", async () => {
    const { sessionToken, whoami } = await signupOwner();
    const billing = billingClient({ sessionToken });
    const resp = await billing.listInvoices({
      billingAccountId: whoami.billingAccountId,
    });
    expect(resp.invoices).toEqual([]);
  });

  it("non-members cannot list another account's invoices", async () => {
    const ownerA = await signupOwner();
    const ownerB = await signupOwner();
    const billingB = billingClient({ sessionToken: ownerB.sessionToken });
    await expect(
      billingB.listInvoices({ billingAccountId: ownerA.whoami.billingAccountId }),
    ).rejects.toMatchObject({ code: Code.PermissionDenied });
  });
});

describe("BillingService — SSO configuration enforcement", () => {
  it("configure SSO with required=true blocks password login for non-break-glass", async () => {
    const owner = await signupOwner();
    const billing = billingClient({ sessionToken: owner.sessionToken });
    await billing.configureSso({
      billingAccountId: owner.whoami.billingAccountId,
      idpId: "acme-okta",
      kind: SsoKind.OIDC,
      loginUrl: "https://acme.okta.com/login",
      required: true,
    });
    // Owner = configurer = break-glass user, so password login still works.
    const me = await authClient().login({
      email: owner.email,
      password: owner.password,
    });
    expect(me.sessionToken).toBeTruthy();
  });

  // ConfigureSso pins the configurer as break-glass. With only public RPCs,
  // you can't get a user's own personal billing into "SSO required, someone
  // else is break-glass" state — so we test the enforcement via SwitchContext.
  it("non-break-glass password user cannot SwitchContext into an SSO-required billing", async () => {
    const owner = await signupOwner();
    const ownerBilling = billingClient({ sessionToken: owner.sessionToken });

    const inviteeEmail = uniqueEmail("ssomember");
    const invite = await ownerBilling.inviteMember({
      billingAccountId: owner.whoami.billingAccountId,
      email: inviteeEmail,
      role: Role.MEMBER,
    });
    const newcomer = await authClient().signup({
      email: inviteeEmail,
      password: "another-very-long-password",
      inviteToken: invite.inviteTokenForDemo,
    });

    await ownerBilling.configureSso({
      billingAccountId: owner.whoami.billingAccountId,
      idpId: "acme-okta",
      kind: SsoKind.OIDC,
      loginUrl: "https://acme.okta.com/login",
      required: true,
    });

    // Invitee's session was minted with auth_method=password and they are not
    // the break-glass user, so entering the SSO-required scope must be refused.
    const newcomerAuth = authClient({ sessionToken: newcomer.sessionToken });
    await expect(
      newcomerAuth.switchContext({ billingAccountId: owner.whoami.billingAccountId }),
    ).rejects.toMatchObject({ code: Code.PermissionDenied });
  });
});
