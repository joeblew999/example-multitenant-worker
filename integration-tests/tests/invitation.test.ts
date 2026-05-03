import { describe, it, expect } from "vitest";
import { Code } from "@connectrpc/connect";
import {
  authClient,
  billingClient,
  invitationClient,
  orgClient,
  signupAndGetToken,
  uniqueEmail,
} from "./helpers";
import { Role as BillingRole, SsoKind } from "../gen/workers/billing/v1/billing_pb.js";
import {
  InvitationStatus,
  Role as InvRole,
  ScopeKind,
} from "../gen/workers/invitation/v1/invitation_pb.js";
import { AuthMethodKind } from "../gen/workers/auth/v1/auth_pb.js";
import { Role as OrgRole } from "../gen/workers/org/v1/org_pb.js";

describe("InvitationService.AcceptInvitation (existing user)", () => {
  it("preview, accept, and verify membership; replay rejected", async () => {
    const owner = await signupAndGetToken(uniqueEmail("owner"));
    const billing = billingClient({ sessionToken: owner.sessionToken });

    const invitee = await signupAndGetToken(uniqueEmail("existing"));
    const inviteeAuth = authClient({ sessionToken: invitee.sessionToken });

    const invite = await billing.inviteMember({
      billingAccountId: owner.whoami.billingAccountId,
      email: invitee.email,
      role: BillingRole.MEMBER,
    });

    const inv = invitationClient({ sessionToken: invitee.sessionToken });
    const preview = await inv.getInvitation({
      token: invite.inviteTokenForDemo,
    });
    expect(preview.invitation?.scopeId).toBe(owner.whoami.billingAccountId);
    expect(preview.invitation?.email.toLowerCase()).toBe(invitee.email.toLowerCase());
    expect(preview.invitation?.status).toBe(InvitationStatus.PENDING);

    const accepted = await inv.acceptInvitation({
      token: invite.inviteTokenForDemo,
    });
    expect(accepted.scopeKind).toBe(ScopeKind.BILLING);
    expect(accepted.scopeId).toBe(owner.whoami.billingAccountId);
    expect(accepted.role).toBe(InvRole.MEMBER);

    // Re-presenting the now-consumed nonce must fail.
    await expect(inv.acceptInvitation({ token: invite.inviteTokenForDemo })).rejects.toMatchObject({
      code: Code.FailedPrecondition,
    });

    // Invitee's whoami now reflects the new billing membership.
    const refreshed = (await inviteeAuth.whoami({})).whoami!;
    const ids = refreshed.billingMemberships.map((m) => m.scopeId);
    expect(ids).toContain(owner.whoami.billingAccountId);
  });

  it("accept rejects mismatched email between session and invite", async () => {
    const owner = await signupAndGetToken(uniqueEmail("owner"));
    const billing = billingClient({ sessionToken: owner.sessionToken });
    const invite = await billing.inviteMember({
      billingAccountId: owner.whoami.billingAccountId,
      email: uniqueEmail("intended"),
      role: BillingRole.MEMBER,
    });
    // Some other user (not the intended invitee) tries to accept.
    const intruder = await signupAndGetToken(uniqueEmail("intruder"));
    const intruderInv = invitationClient({ sessionToken: intruder.sessionToken });
    await expect(
      intruderInv.acceptInvitation({ token: invite.inviteTokenForDemo }),
    ).rejects.toMatchObject({ code: Code.PermissionDenied });
  });
});

describe("InvitationService.ListPendingInvitations", () => {
  it("only lists invitations matching the caller's email", async () => {
    const owner = await signupAndGetToken(uniqueEmail("owner"));
    const billing = billingClient({ sessionToken: owner.sessionToken });

    const targetA = await signupAndGetToken(uniqueEmail("target-a"));
    const targetB = await signupAndGetToken(uniqueEmail("target-b"));
    await billing.inviteMember({
      billingAccountId: owner.whoami.billingAccountId,
      email: targetA.email,
      role: BillingRole.MEMBER,
    });
    await billing.inviteMember({
      billingAccountId: owner.whoami.billingAccountId,
      email: targetB.email,
      role: BillingRole.MEMBER,
    });

    const aInv = invitationClient({ sessionToken: targetA.sessionToken });
    const list = await aInv.listPendingInvitations({});
    const emails = list.invitations.map((i) => i.email.toLowerCase());
    expect(emails).toContain(targetA.email.toLowerCase());
    expect(emails).not.toContain(targetB.email.toLowerCase());
  });
});

describe("InvitationService.{Resend,Revoke}Invitation", () => {
  it("revoke invalidates the outstanding token", async () => {
    const owner = await signupAndGetToken(uniqueEmail("owner"));
    const billing = billingClient({ sessionToken: owner.sessionToken });
    const inviteeEmail = uniqueEmail("revoked");
    const issued = await billing.inviteMember({
      billingAccountId: owner.whoami.billingAccountId,
      email: inviteeEmail,
      role: BillingRole.MEMBER,
    });
    const inv = invitationClient({ sessionToken: owner.sessionToken });
    await inv.revokeInvitation({ invitationId: issued.invitationId });
    // Token rotated → preview now reports the nonce mismatch.
    await expect(inv.getInvitation({ token: issued.inviteTokenForDemo })).rejects.toMatchObject({
      code: Code.FailedPrecondition,
    });
  });

  it("resend rotates the token without changing the invitation id", async () => {
    const owner = await signupAndGetToken(uniqueEmail("owner"));
    const billing = billingClient({ sessionToken: owner.sessionToken });
    const inviteeEmail = uniqueEmail("resent");
    const first = await billing.inviteMember({
      billingAccountId: owner.whoami.billingAccountId,
      email: inviteeEmail,
      role: BillingRole.MEMBER,
    });
    const inv = invitationClient({ sessionToken: owner.sessionToken });
    const resent = await inv.resendInvitation({
      invitationId: first.invitationId,
    });
    expect(resent.invitation?.id).toBe(first.invitationId);
    expect(resent.inviteTokenForDemo).not.toBe(first.inviteTokenForDemo);
    // The newly-resent token works.
    const preview = await inv.getInvitation({
      token: resent.inviteTokenForDemo,
    });
    expect(preview.invitation?.email.toLowerCase()).toBe(inviteeEmail.toLowerCase());
  });
});

describe("InvitationService.DeclineInvitation", () => {
  it("decline marks the invitation declined; replays fail", async () => {
    const owner = await signupAndGetToken(uniqueEmail("owner"));
    const billing = billingClient({ sessionToken: owner.sessionToken });
    const target = await signupAndGetToken(uniqueEmail("decliner"));
    const issued = await billing.inviteMember({
      billingAccountId: owner.whoami.billingAccountId,
      email: target.email,
      role: BillingRole.MEMBER,
    });
    const inv = invitationClient({ sessionToken: target.sessionToken });
    await inv.declineInvitation({ token: issued.inviteTokenForDemo });
    await expect(inv.declineInvitation({ token: issued.inviteTokenForDemo })).rejects.toMatchObject(
      { code: Code.FailedPrecondition },
    );
  });
});

describe("InvitationService — SSO-required invitation acceptance", () => {
  it("rejects a password-session acceptance when the invitation requires SSO", async () => {
    const owner = await signupAndGetToken(uniqueEmail("ssoowner"));
    const org = orgClient({ sessionToken: owner.sessionToken });
    const created = await org.createOrganization({
      billingAccountId: owner.whoami.billingAccountId,
      displayName: "SSO-Strict Org",
    });
    await org.configureSso({
      orgId: created.organization!.id,
      idpId: "strict-okta",
      kind: SsoKind.OIDC,
      loginUrl: "https://strict.okta.com/login",
      required: true,
    });

    const inviteeEmail = uniqueEmail("pwdinvitee");
    const invite = await org.inviteMember({
      orgId: created.organization!.id,
      email: inviteeEmail,
      role: OrgRole.MEMBER,
    });

    const invitee = await signupAndGetToken(inviteeEmail);
    const inv = invitationClient({ sessionToken: invitee.sessionToken });
    await expect(inv.acceptInvitation({ token: invite.inviteTokenForDemo })).rejects.toMatchObject({
      code: Code.PermissionDenied,
    });
  });

  it("SSO-authenticated user can accept an SSO-required invitation", async () => {
    const owner = await signupAndGetToken(uniqueEmail("ssoowner"));
    const billing = billingClient({ sessionToken: owner.sessionToken });

    await billing.configureSso({
      billingAccountId: owner.whoami.billingAccountId,
      idpId: "strict-okta",
      kind: SsoKind.OIDC,
      loginUrl: "https://strict.okta.com/login",
      required: false,
    });

    const org = orgClient({ sessionToken: owner.sessionToken });
    const created = await org.createOrganization({
      billingAccountId: owner.whoami.billingAccountId,
      displayName: "SSO-Strict Org",
    });
    await org.configureSso({
      orgId: created.organization!.id,
      idpId: "strict-okta",
      kind: SsoKind.OIDC,
      loginUrl: "https://strict.okta.com/login",
      required: true,
    });

    const inviteeEmail = uniqueEmail("ssoinvitee");
    const invite = await org.inviteMember({
      orgId: created.organization!.id,
      email: inviteeEmail,
      role: OrgRole.MEMBER,
    });

    await signupAndGetToken(inviteeEmail);

    const auth = authClient();
    const start = await auth.ssoStart({
      scopeHint: `billing:${owner.whoami.billingAccountId}`,
    });
    const complete = await auth.ssoComplete({
      state: start.state,
      idpUserId: "okta|ssoinvitee-123",
      email: inviteeEmail,
      idpId: "strict-okta",
    });
    expect(complete.whoami?.authMethod?.kind).toBe(AuthMethodKind.SSO);
    expect(complete.whoami?.authMethod?.idpId).toBe("strict-okta");

    const inv = invitationClient({ sessionToken: complete.sessionToken });
    const accepted = await inv.acceptInvitation({ token: invite.inviteTokenForDemo });
    expect(accepted.scopeKind).toBe(ScopeKind.ORG);
    expect(accepted.scopeId).toBe(created.organization!.id);
    expect(accepted.role).toBe(InvRole.MEMBER);
  });
});

describe("AuthService.SsoStart / SsoComplete (mocked IdP)", () => {
  it("SsoStart returns redirect URL only when SSO is configured; SsoComplete creates a JIT user", async () => {
    const owner = await signupAndGetToken(uniqueEmail("ssoowner"));
    const billing = billingClient({ sessionToken: owner.sessionToken });
    await billing.configureSso({
      billingAccountId: owner.whoami.billingAccountId,
      idpId: "acme-okta",
      kind: SsoKind.OIDC,
      loginUrl: "https://acme.okta.com/login",
      required: false,
    });

    const auth = authClient();
    const start = await auth.ssoStart({
      scopeHint: `billing:${owner.whoami.billingAccountId}`,
    });
    expect(start.redirectUrl).toContain("acme.okta.com/login");
    expect(start.state).toBeTruthy();

    const newUserEmail = uniqueEmail("ssojit");
    const complete = await auth.ssoComplete({
      state: start.state,
      idpUserId: "okta|new-user-123",
      email: newUserEmail,
      idpId: "acme-okta",
    });
    expect(complete.jitCreated).toBe(true);
    expect(complete.sessionToken).toBeTruthy();
    expect(complete.whoami?.email).toBe(newUserEmail);
    expect(complete.whoami?.authMethod?.kind).toBe(AuthMethodKind.SSO);
    expect(complete.whoami?.authMethod?.idpId).toBe("acme-okta");
  });

  it("SsoStart fails with NotFound when no SSO is configured", async () => {
    const owner = await signupAndGetToken(uniqueEmail("nosso"));
    const auth = authClient();
    await expect(
      auth.ssoStart({ scopeHint: `billing:${owner.whoami.billingAccountId}` }),
    ).rejects.toMatchObject({ code: Code.NotFound });
  });
});
