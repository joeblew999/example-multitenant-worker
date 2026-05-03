import { createClient, type Client, type Transport } from "@connectrpc/connect";
import { createConnectTransport } from "@connectrpc/connect-web";
import type { DescService } from "@bufbuild/protobuf";

import { mf, mfUrl } from "./mf";

import { AuthService } from "../gen/workers/auth/v1/auth_pb.js";
import { BillingService } from "../gen/workers/billing/v1/billing_pb.js";
import { OrgService } from "../gen/workers/org/v1/org_pb.js";
import { InvitationService } from "../gen/workers/invitation/v1/invitation_pb.js";

export interface TransportOptions {
  /** Bearer token for the Authorization header. */
  sessionToken?: string;
  /** false → JSON codec, true (default) → binary protobuf. */
  useBinaryFormat?: boolean;
  /** Extra headers attached to every request from this transport. */
  defaultHeaders?: Record<string, string>;
}

export function makeTransport(opts: TransportOptions = {}): Transport {
  return createConnectTransport({
    baseUrl: mfUrl,
    useBinaryFormat: opts.useBinaryFormat ?? true,
    fetch: ((input, init) => {
      const headers = new Headers(init?.headers);
      if (opts.sessionToken && !headers.has("authorization")) {
        headers.set("authorization", `Bearer ${opts.sessionToken}`);
      }
      for (const [k, v] of Object.entries(opts.defaultHeaders ?? {})) {
        if (!headers.has(k)) headers.set(k, v);
      }
      return mf.dispatchFetch(input as string, { ...init, headers });
    }) as typeof globalThis.fetch,
  });
}

function clientFor<S extends DescService>(service: S, opts?: TransportOptions): Client<S> {
  return createClient(service, makeTransport(opts));
}

export const authClient = (opts?: TransportOptions) => clientFor(AuthService, opts);
export const billingClient = (opts?: TransportOptions) => clientFor(BillingService, opts);
export const orgClient = (opts?: TransportOptions) => clientFor(OrgService, opts);
export const invitationClient = (opts?: TransportOptions) => clientFor(InvitationService, opts);

/** Generate a unique email per test so each test starts from a clean slate
 *  (the miniflare D1 instance is shared across the run). */
let counter = 0;
export function uniqueEmail(prefix = "user") {
  counter += 1;
  return `${prefix}+${Date.now()}-${counter}@example.com`;
}

export async function signupAndGetToken(
  email = uniqueEmail(),
  password = "verylong-password-is-fine",
) {
  const auth = authClient();
  const resp = await auth.signup({ email, password });
  return { email, password, sessionToken: resp.sessionToken, whoami: resp.whoami! };
}
