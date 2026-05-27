import { ConnectError, createClient } from "@connectrpc/connect";
import { createConnectTransport } from "@connectrpc/connect-web";

import { AuthService } from "../gen/workers/auth/v1/auth_pb.js";
import { BillingService } from "../gen/workers/billing/v1/billing_pb.js";
import { InvitationService } from "../gen/workers/invitation/v1/invitation_pb.js";
import { OrgService } from "../gen/workers/org/v1/org_pb.js";

export const SESSION_STORAGE_KEY = "wm.session";

export function errorMessage(e: unknown, fallback: string): string {
  return e instanceof ConnectError ? e.message : fallback;
}

// Auth context owns the storage schema and pushes the current token here so
// the transport can stamp Authorization headers without re-parsing on every
// request.
let currentToken: string | null = null;

export function setAuthToken(token: string | null) {
  currentToken = token;
}

const transport = createConnectTransport({
  baseUrl: typeof window !== "undefined" ? window.location.origin : "/",
  fetch: ((input, init) => {
    const headers = new Headers(init?.headers);
    if (!headers.has("Authorization") && currentToken) {
      headers.set("Authorization", `Bearer ${currentToken}`);
    }
    return globalThis.fetch(input as RequestInfo, { ...init, headers });
  }) as typeof globalThis.fetch,
});

export const authClient = createClient(AuthService, transport);
export const billingClient = createClient(BillingService, transport);
export const invitationClient = createClient(InvitationService, transport);
export const orgClient = createClient(OrgService, transport);
