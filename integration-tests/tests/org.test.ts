import { describe, it, expect } from "vitest";
import { Code } from "@connectrpc/connect";
import { authClient, billingClient, orgClient, signupAndGetToken, uniqueEmail } from "./helpers";
import { Role } from "../gen/workers/org/v1/org_pb.js";
import { SsoKind } from "../gen/workers/billing/v1/billing_pb.js";

async function signupOwner() {
  const result = await signupAndGetToken(uniqueEmail("orgowner"));
  return { ...result, personalBillingId: result.whoami.billingAccountId };
}

describe("OrgService — create / list / get", () => {
  it("creates an org under the caller's billing and shows up in lists", async () => {
    const { sessionToken, personalBillingId } = await signupOwner();
    const org = orgClient({ sessionToken });
    const created = await org.createOrganization({
      billingAccountId: personalBillingId,
      displayName: "Engineering",
    });
    expect(created.organization?.displayName).toBe("Engineering");
    expect(created.organization?.billingAccountId).toBe(personalBillingId);

    const list = await org.listOrganizations({
      billingAccountId: personalBillingId,
    });
    const ids = list.organizations.map((o) => o.id);
    expect(ids).toContain(created.organization!.id);

    const got = await org.getOrganization({ id: created.organization!.id });
    expect(got.organization?.displayName).toBe("Engineering");
  });

  it("non-owners can't create orgs", async () => {
    const a = await signupOwner();
    const b = await signupOwner();
    const orgA = orgClient({ sessionToken: a.sessionToken });
    const orgB = orgClient({ sessionToken: b.sessionToken });
    await expect(
      orgB.createOrganization({
        billingAccountId: a.personalBillingId,
        displayName: "Forbidden",
      }),
    ).rejects.toMatchObject({ code: Code.PermissionDenied });
    // Sanity: A can.
    const ok = await orgA.createOrganization({
      billingAccountId: a.personalBillingId,
      displayName: "OK",
    });
    expect(ok.organization?.displayName).toBe("OK");
  });

  it("delete org removes it from listings", async () => {
    const { sessionToken, personalBillingId } = await signupOwner();
    const org = orgClient({ sessionToken });
    const created = await org.createOrganization({
      billingAccountId: personalBillingId,
      displayName: "DeleteMe",
    });
    await org.deleteOrganization({ orgId: created.organization!.id });
    await expect(org.getOrganization({ id: created.organization!.id })).rejects.toMatchObject({
      code: Code.NotFound,
    });
  });
});

describe("OrgService — cross-tenant isolation", () => {
  it("getOrganization on another tenant's org is PermissionDenied", async () => {
    const a = await signupOwner();
    const b = await signupOwner();
    const orgA = orgClient({ sessionToken: a.sessionToken });
    const created = await orgA.createOrganization({
      billingAccountId: a.personalBillingId,
      displayName: "Secret",
    });

    const orgB = orgClient({ sessionToken: b.sessionToken });
    await expect(orgB.getOrganization({ id: created.organization!.id })).rejects.toMatchObject({
      code: Code.PermissionDenied,
    });
  });

  it("listOrganizations scoped to another tenant's billing is PermissionDenied", async () => {
    const a = await signupOwner();
    const b = await signupOwner();
    const orgB = orgClient({ sessionToken: b.sessionToken });
    await expect(
      orgB.listOrganizations({ billingAccountId: a.personalBillingId }),
    ).rejects.toMatchObject({ code: Code.PermissionDenied });
  });
});

describe("OrgService — SSO configuration", () => {
  it("configures SSO at org level and SsoStart resolves it", async () => {
    const owner = await signupOwner();
    const org = orgClient({ sessionToken: owner.sessionToken });
    const created = await org.createOrganization({
      billingAccountId: owner.personalBillingId,
      displayName: "SSO Org",
    });
    const ssoResp = await org.configureSso({
      orgId: created.organization!.id,
      idpId: "org-okta",
      kind: SsoKind.OIDC,
      loginUrl: "https://org.okta.com/login",
      required: false,
    });
    expect(ssoResp.sso?.idpId).toBe("org-okta");

    const auth = authClient();
    const start = await auth.ssoStart({
      scopeHint: `org:${created.organization!.id}`,
    });
    expect(start.redirectUrl).toContain("org.okta.com/login");
    expect(start.state).toBeTruthy();
  });

  it("org-level SSO takes precedence over billing-level for SsoStart", async () => {
    const owner = await signupOwner();
    const billing = billingClient({ sessionToken: owner.sessionToken });
    await billing.configureSso({
      billingAccountId: owner.personalBillingId,
      idpId: "billing-okta",
      kind: SsoKind.OIDC,
      loginUrl: "https://billing.okta.com/login",
      required: false,
    });
    const org = orgClient({ sessionToken: owner.sessionToken });
    const created = await org.createOrganization({
      billingAccountId: owner.personalBillingId,
      displayName: "Precedence Org",
    });
    await org.configureSso({
      orgId: created.organization!.id,
      idpId: "org-okta",
      kind: SsoKind.OIDC,
      loginUrl: "https://org.okta.com/login",
      required: false,
    });

    const auth = authClient();
    const start = await auth.ssoStart({
      scopeHint: `org:${created.organization!.id}`,
    });
    expect(start.redirectUrl).toContain("org.okta.com/login");
    expect(start.redirectUrl).not.toContain("billing.okta.com");
  });
});

describe("OrgService — invite + accept", () => {
  it("creator invites another user; invitee accepts via signup flow and joins as a member", async () => {
    const { sessionToken, personalBillingId } = await signupOwner();
    const org = orgClient({ sessionToken });

    const orgRow = await org.createOrganization({
      billingAccountId: personalBillingId,
      displayName: "Eng",
    });
    const orgId = orgRow.organization!.id;

    const inviteeEmail = uniqueEmail("orginvite");
    const invite = await org.inviteMember({
      orgId,
      email: inviteeEmail,
      role: Role.MEMBER,
    });
    expect(invite.inviteTokenForDemo).toBeTruthy();

    const auth = authClient();
    const signup = await auth.signup({
      email: inviteeEmail,
      password: "another-very-long-password",
      inviteToken: invite.inviteTokenForDemo,
    });
    expect(signup.sessionToken).toBeTruthy();

    const newcomerAuth = authClient({ sessionToken: signup.sessionToken });
    const w = (await newcomerAuth.whoami({})).whoami!;
    const orgIds = w.orgMemberships.map((m) => m.scopeId);
    expect(orgIds).toContain(orgId);
  });

  it("removing an org member drops them from membership", async () => {
    const owner = await signupOwner();
    const ownerOrg = orgClient({ sessionToken: owner.sessionToken });
    const orgRow = await ownerOrg.createOrganization({
      billingAccountId: owner.personalBillingId,
      displayName: "Eng",
    });
    const orgId = orgRow.organization!.id;
    const inviteeEmail = uniqueEmail("toremove");
    const invite = await ownerOrg.inviteMember({
      orgId,
      email: inviteeEmail,
      role: Role.MEMBER,
    });
    const signup = await authClient().signup({
      email: inviteeEmail,
      password: "another-very-long-password",
      inviteToken: invite.inviteTokenForDemo,
    });
    const inviteeUserId = signup.whoami!.userId;

    await ownerOrg.removeMember({ orgId, userId: inviteeUserId });

    const inviteeAuth = authClient({ sessionToken: signup.sessionToken });
    const w = (await inviteeAuth.whoami({})).whoami!;
    const orgIds = w.orgMemberships.map((m) => m.scopeId);
    expect(orgIds).not.toContain(orgId);
  });
});
