# example-multitenant-worker

Multi-tenant SaaS auth on Cloudflare Workers. Sessions, invites, billing
account membership, organizations, SSO, and password reset, all working.
Same layout as
[`example-connectrpc-worker`](https://github.com/connyay/example-connectrpc-worker).
Authorization uses [`libmacaroon`](https://crates.io/crates/libmacaroon),
so verification is a signature check plus caveat checks. The hot path
never touches the DB.

## At a glance

```
fetch (event handler)
  ├─ routes::try_handle        ← /healthz, /oauth/callback, /verify-email
  └─ AuthLayer                 ← optional: verifies session macaroon, inserts SessionContext
      └─ ConnectRpcService     ← parses path, dispatches by service+method
            └─ Router          ← AuthService, BillingService, OrgService, InvitationService
                └─ Repo (supertrait of UserRepo, BillingRepo, OrgRepo, MembershipRepo,
                │        SsoConfigRepo, InvitationRepo, AuthFlowRepo)
                └─ BillingProvider (D1 in production, in-memory in tests)
```

Four ConnectRPC services under `proto/workers/`:

- `auth/v1` -- signup, login, SSO, link-identity, change-password,
  request/consume password reset, switch-context (sliding refresh),
  whoami.
- `billing/v1` -- get / list billing accounts, member ops, payment method,
  subscription ops, SSO config, auto-join domain, delete-account.
- `org/v1` -- create / get / list / delete orgs, member ops, org-level
  SSO.
- `invitation/v1` -- get/accept/decline invitations, list pending,
  resend, revoke.

## Macaroons

Every auth artifact is a [`libmacaroon`](https://crates.io/crates/libmacaroon)
macaroon signed by a single 32-byte root key. Tokens carry a
`purpose=<session|invitation|password_reset|email_verify>` caveat, so a
session can never be used as an invite (or vice versa); the verifier
rejects on mismatched purpose.

| Token kind     | Caveats                                                                                                             | TTL    |
| -------------- | ------------------------------------------------------------------------------------------------------------------- | ------ |
| Session        | `purpose=session`, `user`, `email`, `billing`, `role`, `auth_method`, `exp` (+ optional `org`)                      | 24h    |
| Invitation     | `purpose=invitation`, `invitation_id`, `email`, `scope_kind`, `scope_id`, `role`, `nonce`, `exp` (+ `required_idp`) | 7d     |
| Password reset | `purpose=password_reset`, `user`, `nonce`, `exp`                                                                    | 15 min |
| Email verify   | `purpose=email_verify`, `user`, `nonce`, `exp`                                                                      | 24h    |

One-shot tokens (invite, reset, verify) carry a `nonce` consumed in the
`consumed_nonces` table on use. That's the only DB interaction tied to a
macaroon, and it happens on the use RPC, not during verify. For sliding
refresh, clients call `Auth.SwitchContext` to re-mint a session against
the same root key.

The session `Authorization: Bearer <token>` header is verified by the
`AuthLayer` tower middleware ahead of dispatch. Handlers extract the
verified `SessionContext` via `require_session(ctx)?`.

## Layout

```
proto/workers/{auth,billing,org,invitation}/v1/*.proto    # service definitions
build.rs                                                  # connectrpc-build driver
src/lib.rs                                                # fetch handler, env wiring
src/middleware/auth.rs                                    # tower layer
src/routes.rs                                             # /healthz, /oauth/callback, /verify-email
src/auth/{keyring, tokens, password, session}.rs          # macaroon mint/verify, Argon2id
src/billing/provider.rs                                   # BillingProvider trait + impls
src/domain/{ids, enums, entities}.rs                      # typed IDs, role/scope enums, structs
src/store/repo/{mod, user, billing, org, membership, …}.rs # sub-traits + Repo supertrait
src/store/{mem, d1, error}.rs                              # InMemory + D1 impls, StoreError
src/services/{auth, billing, org, invitation}.rs          # service handlers
src/services/{authz, session, common, convert}.rs         # shared helpers
src/{state, time}.rs                                      # AppState + Config; Clock trait
migrations/0001_init.sql                                  # full schema
wrangler.toml                                             # binding + env config
integration-tests/                                        # vitest + miniflare + Connect-ES
web/                                                      # React SPA (Vite + Connect-ES)
```

## Data model

A **User** is created by signup (any method). `email_verified=false` on
password signup, `true` on SSO JIT since the IdP already verified.

An **Identity** is a `(provider, provider_user_id)` unique credential on
a user. Providers: `password`, `github`, `google`, `sso:<idp_id>`.
Linking is always explicit; we never auto-link on email match.

Every user gets a personal **BillingAccount** on signup (payment + SSO
scope parent). Personal accounts can't be deleted while the user exists.

An **Organization** belongs to exactly one billing account. No transfers,
just delete and recreate.

**Membership** lives in two tables (`billing_memberships`,
`org_memberships`). They're orthogonal -- billing account ownership does
not grant implicit org access.

## Store traits

The `Repo` supertrait composes seven focused sub-traits, one per
aggregate boundary. Implementations (`D1Repo`, `InMemoryRepo`) implement
each sub-trait independently; the blanket impl auto-derives `Repo` for
any type satisfying all seven. Helper functions bind on only the
sub-traits they need.

| Trait            | Methods | Scope                                          |
| ---------------- | ------- | ---------------------------------------------- |
| `UserRepo`       | 11      | User/identity CRUD + atomic signup paths       |
| `BillingRepo`    | 9       | Billing account CRUD and counts                |
| `OrgRepo`        | 6       | Organization CRUD                              |
| `MembershipRepo` | 12      | Billing + org membership ops and owner counts  |
| `SsoConfigRepo`  | 4       | SSO configuration per scope                    |
| `InvitationRepo` | 7       | Invitation lifecycle                           |
| `AuthFlowRepo`   | 3       | Nonces + SSO state (ephemeral auth-flow tokens)|

## SSO precedence

When configured, SSO is required for the protected scope. Org-level
config wins over billing-level. The user who configures SSO is recorded
as the break-glass user and keeps password access. Invites freeze the
required IdP at issuance time so subsequent SSO config changes don't
re-target outstanding invites.

The demo's SSO is mocked: `Auth.SsoStart` returns the configured login
URL with a per-flow `state` token, and `Auth.SsoComplete(state, idp_user_id, email, idp_id)`
simulates the IdP callback. The `/oauth/callback` HTTP route is a stub.
A production deployment would exchange the code with the IdP and then
call `Auth.SsoComplete` server side.

## Web frontend

A React SPA lives in `web/`. Vite, React Router, and Connect-ES talking
to the worker over ConnectRPC.

```sh
cd web && pnpm install && pnpm dev    # dev server (proxies to wrangler dev)
cd web && pnpm build                  # production build
```

## Running tests

Two layers, mirroring `example-connectrpc-worker`:

```sh
# Native unit tests (handlers + InMemoryRepo + InMemoryBillingProvider).
cargo test

# Integration tests: builds the wasm worker (installs worker-build if needed),
# loads it under miniflare with a real D1 binding, exercises every service
# through a Connect-ES client.
cd integration-tests && pnpm install && pnpm test
```

Native unit tests cover macaroon mint/verify roundtrips, password
hashing, repo invariants (last-owner protection, personal billing
non-deletability, nonce replay rejection), and handler flows (signup to
login, signup via invite to membership, change-password, context switch,
org creation).

Integration tests cover the full stack against the wasm worker:
non-RPC routes, signup/login/whoami/change-password/password-reset,
billing member ops + subscription lifecycle, SSO configuration,
org create/delete + invite flow, invitation accept/decline/revoke/resend.

## Building for Cloudflare

```sh
cargo install worker-build@^0.8               # one-time: install the build tool
cargo check --target wasm32-unknown-unknown   # type-check the wasm build
worker-build --release                        # produce build/index.{js,_bg.wasm}
wrangler dev                                  # local dev
wrangler deploy                               # ship it
```

Apply the D1 schema before first use:

```sh
wrangler d1 migrations apply workers-multitenant
```

## Configuration

Read at fetch time from environment vars (configured in `wrangler.toml`):

| Var                          | Default    | Purpose                                   |
| ---------------------------- | ---------- | ----------------------------------------- |
| `SESSION_KEY`                | dev-key    | base64-encoded 32-byte macaroon root key. |
| `SESSION_TTL_SECONDS`        | 86400      | Session macaroon TTL.                     |
| `INVITATION_TTL_SECONDS`     | 604800     | Invite TTL (7d).                          |
| `PASSWORD_RESET_TTL_SECONDS` | 900        | Password-reset TTL (15m).                 |
| `EMAIL_VERIFY_TTL_SECONDS`   | 86400      | Email-verify TTL (24h).                   |
| `SSO_STATE_TTL_SECONDS`      | 600        | TTL of an `Auth.SsoStart` state token.    |
| `ENFORCE_EMAIL_VERIFICATION` | "" (false) | When `"true"`, blocks unverified users.   |

When `SESSION_KEY` is empty, the worker boots with a deterministic dev
key (logged on startup) so `wrangler dev` works without ceremony. Don't
ship the dev key to production. Generate a real one with
`openssl rand -base64 32`.

## Not in scope

Left out of the demo on purpose. Extension points noted where relevant:

- Audit log: a write-through table on every state-changing RPC. The
  easiest seam is a Tower layer on top of `ConnectRpcService` that
  inspects the dispatched method and writes to a new `audit_log` table.
- Real Stripe / Lago: implement `BillingProvider` against your vendor of
  choice. The trait surface (`get_subscription`, `set_payment_method`,
  `cancel_subscription`, `list_invoices`, `record_invoice`,
  `ensure_subscription`) is the seam.
- Email: we auto-verify on SSO signup and leave password signup
  unverified. `ENFORCE_EMAIL_VERIFICATION` is wired but off. Plumb a
  `Mailer` trait and an `Auth.IssueEmailVerifyToken` RPC when wiring in
  real email.
- Membership version pinning on session macaroons: sessions stay valid
  until `exp` even if a member is removed. To tighten, add a `mver=<n>`
  caveat with a per-(scope, user) version that the verifier cross-checks
  (or an explicit `revoked_sessions` table). The demo accepts this
  stale-token window.
- Custom roles / RBAC: `owner` / `member` only.
- Service-to-service / API tokens: sessions only.
- Org to billing transfer: orgs are immutable to their parent.
- Soft delete + recovery: hard delete only, no sweeper.
- Atomicity on multi-row writes: D1 batches are atomic per CF docs, so
  we use them for signup, billing account creation, and invitation
  acceptance. If a non-batch sequence dies mid-flight (worker eviction),
  partial state is possible. The `consumed_nonces` table makes one-shot
  token state recoverable, but partial signups are best effort.
