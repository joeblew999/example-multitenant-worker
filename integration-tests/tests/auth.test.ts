import { describe, it, expect } from "vitest";
import { Code, ConnectError } from "@connectrpc/connect";
import { authClient, signupAndGetToken, uniqueEmail } from "./helpers";
import { AuthMethodKind, Role } from "../gen/workers/auth/v1/auth_pb.js";

describe("AuthService.Signup", () => {
  it("creates user, identity, personal billing + personal org — returns session", async () => {
    const auth = authClient();
    const email = uniqueEmail("ada");
    const resp = await auth.signup({ email, password: "verylong-password" });
    expect(resp.sessionToken).toBeTruthy();
    const w = resp.whoami!;
    expect(w.email).toBe(email);
    expect(w.role).toBe(Role.OWNER);
    expect(w.authMethod?.kind).toBe(AuthMethodKind.PASSWORD);
    expect(w.billingAccountId).toBeTruthy();
    expect(w.orgId).toBe("");
    // The user gets one personal billing account and one personal org on signup.
    expect(w.billingMemberships.length).toBe(1);
    expect(w.billingMemberships[0].displayName).toContain("personal");
    expect(w.orgMemberships.length).toBe(1);
    expect(w.orgMemberships[0].displayName).toContain("personal");
    expect(w.orgMemberships[0].role).toBe(Role.OWNER);
  });

  it("rejects short passwords with InvalidArgument", async () => {
    const auth = authClient();
    await expect(auth.signup({ email: uniqueEmail(), password: "short" })).rejects.toMatchObject({
      name: "ConnectError",
      code: Code.InvalidArgument,
    });
  });

  it("rejects duplicate emails (case-insensitive) with AlreadyExists", async () => {
    const auth = authClient();
    const email = uniqueEmail("dup");
    await auth.signup({ email, password: "verylong-password" });
    await expect(
      auth.signup({
        email: email.toUpperCase(),
        password: "another-very-long-password",
      }),
    ).rejects.toMatchObject({ code: Code.AlreadyExists });
  });
});

describe("AuthService.Login", () => {
  it("returns a session for correct credentials", async () => {
    const auth = authClient();
    const { email, password } = await signupAndGetToken();
    const resp = await auth.login({ email, password });
    expect(resp.sessionToken).toBeTruthy();
    expect(resp.whoami?.email).toBe(email);
  });

  it("rejects wrong password with Unauthenticated", async () => {
    const auth = authClient();
    const { email } = await signupAndGetToken();
    await expect(auth.login({ email, password: "wrong-password" })).rejects.toMatchObject({
      code: Code.Unauthenticated,
    });
  });

  it("rejects unknown email with Unauthenticated", async () => {
    const auth = authClient();
    await expect(
      auth.login({ email: uniqueEmail("ghost"), password: "irrelevant" }),
    ).rejects.toMatchObject({ code: Code.Unauthenticated });
  });
});

describe("AuthService.Whoami", () => {
  it("requires a session token", async () => {
    const auth = authClient();
    await expect(auth.whoami({})).rejects.toMatchObject({
      code: Code.Unauthenticated,
    });
  });

  it("echoes the verified session", async () => {
    const { email, sessionToken } = await signupAndGetToken();
    const auth = authClient({ sessionToken });
    const resp = await auth.whoami({});
    expect(resp.whoami?.email).toBe(email);
    expect(resp.whoami?.role).toBe(Role.OWNER);
  });
});

describe("AuthService.ChangePassword", () => {
  it("requires the old password to match", async () => {
    const { sessionToken } = await signupAndGetToken();
    const auth = authClient({ sessionToken });
    await expect(
      auth.changePassword({
        oldPassword: "wrong-old",
        newPassword: "another-very-long-password",
      }),
    ).rejects.toMatchObject({ code: Code.Unauthenticated });
  });

  it("succeeds with the right old password and the new one logs in", async () => {
    const { email, password, sessionToken } = await signupAndGetToken();
    const auth = authClient({ sessionToken });
    await auth.changePassword({
      oldPassword: password,
      newPassword: "fresh-new-very-long-password",
    });
    // Old password no longer works.
    await expect(authClient().login({ email, password })).rejects.toBeInstanceOf(ConnectError);
    // New one does.
    const resp = await authClient().login({
      email,
      password: "fresh-new-very-long-password",
    });
    expect(resp.sessionToken).toBeTruthy();
  });
});

describe("AuthService.SwitchContext", () => {
  it("re-mints a session at the personal-org scope and reflects in whoami", async () => {
    const { sessionToken } = await signupAndGetToken();
    const auth = authClient({ sessionToken });
    const before = (await auth.whoami({})).whoami!;
    expect(before.orgId).toBe("");
    const personalOrgId = before.orgMemberships[0].scopeId;

    const switched = await auth.switchContext({ orgId: personalOrgId });
    expect(switched.sessionToken).toBeTruthy();
    expect(switched.sessionToken).not.toBe(sessionToken);
    expect(switched.whoami?.orgId).toBe(personalOrgId);
    expect(switched.whoami?.billingAccountId).toBe(before.billingAccountId);

    // The new token actually carries the org caveat.
    const refreshed = (await authClient({ sessionToken: switched.sessionToken }).whoami({}))
      .whoami!;
    expect(refreshed.orgId).toBe(personalOrgId);
  });

  it("rejects switching into a billing account the caller doesn't belong to", async () => {
    const a = await signupAndGetToken(uniqueEmail("a"));
    const b = await signupAndGetToken(uniqueEmail("b"));
    const wA = (await authClient({ sessionToken: a.sessionToken }).whoami({})).whoami!;
    const authB = authClient({ sessionToken: b.sessionToken });
    await expect(
      authB.switchContext({ billingAccountId: wA.billingAccountId }),
    ).rejects.toMatchObject({ code: Code.PermissionDenied });
  });
});

describe("AuthService — bearer-token verification", () => {
  it("rejects a tampered session token with Unauthenticated", async () => {
    const { sessionToken } = await signupAndGetToken();
    // Flip the trailing characters — the macaroon signature won't verify.
    const tampered = `${sessionToken.slice(0, -4)}AAAA`;
    const auth = authClient({ sessionToken: tampered });
    await expect(auth.whoami({})).rejects.toMatchObject({
      code: Code.Unauthenticated,
    });
  });

  it("rejects an arbitrary non-token bearer with Unauthenticated", async () => {
    const auth = authClient({ sessionToken: "not-a-real-macaroon" });
    await expect(auth.whoami({})).rejects.toMatchObject({
      code: Code.Unauthenticated,
    });
  });
});

describe("AuthService.LinkIdentity", () => {
  it("links a GitHub identity to an existing password user", async () => {
    const { sessionToken } = await signupAndGetToken();
    const auth = authClient({ sessionToken });
    const resp = await auth.linkIdentity({ provider: "github", code: "gh-user-42" });
    expect(resp.identityId).toBeTruthy();
  });

  it("links an SSO identity using the sso:<idp> provider format", async () => {
    const { sessionToken } = await signupAndGetToken();
    const auth = authClient({ sessionToken });
    const resp = await auth.linkIdentity({ provider: "sso:acme-okta", code: "okta|user-99" });
    expect(resp.identityId).toBeTruthy();
  });

  it("requires an authenticated session", async () => {
    const auth = authClient();
    await expect(
      auth.linkIdentity({ provider: "github", code: "gh-user-42" }),
    ).rejects.toMatchObject({ code: Code.Unauthenticated });
  });
});

describe("AuthService.{Request,Consume}PasswordReset", () => {
  it("returns a token for known emails, empty for unknown — and the token resets the password", async () => {
    const { email, password } = await signupAndGetToken();
    const auth = authClient();

    // Unknown email returns OK without a demo token.
    const ghost = await auth.requestPasswordReset({
      email: uniqueEmail("ghost"),
    });
    expect(ghost.resetTokenForDemo ?? "").toBe("");

    const issued = await auth.requestPasswordReset({ email });
    expect(issued.resetTokenForDemo).toBeTruthy();

    // Consume the token, set a new password, get back a fresh session.
    const consume = await auth.consumePasswordReset({
      token: issued.resetTokenForDemo!,
      newPassword: "rotated-very-long-password",
    });
    expect(consume.sessionToken).toBeTruthy();

    // Old password rejected, new one accepted.
    await expect(auth.login({ email, password })).rejects.toBeInstanceOf(ConnectError);
    const ok = await auth.login({
      email,
      password: "rotated-very-long-password",
    });
    expect(ok.sessionToken).toBeTruthy();
  });

  it("rejects re-use of an already-consumed reset token", async () => {
    const { email } = await signupAndGetToken();
    const auth = authClient();
    const issued = await auth.requestPasswordReset({ email });
    await auth.consumePasswordReset({
      token: issued.resetTokenForDemo!,
      newPassword: "rotated-very-long-password",
    });
    await expect(
      auth.consumePasswordReset({
        token: issued.resetTokenForDemo!,
        newPassword: "yet-another-very-long-password",
      }),
    ).rejects.toMatchObject({ code: Code.FailedPrecondition });
  });
});
