-- Multitenant SaaS auth schema.
--
-- Notes on shape:
--   - All ids are TEXT (UUID v4). SQLite stores TEXT efficiently and avoids
--     the i64-via-D1Real shenanigans the example-connectrpc-worker repo
--     warns about.
--   - Email is stored case-preserving but compared case-insensitively.
--     Uniqueness is enforced via a generated lowercased column.
--   - SSO config is per-scope (billing OR org). The two-row design (one
--     row per scope) keeps lookups simple; org-level wins, billing-level
--     is the fallback. Unique on (scope_kind, scope_id).
--   - `consumed_nonces` is the only table touched on a token *use* path.
--     Verification itself never reads it; the use-RPC marks-and-checks.
--   - Memberships live in two tables (billing_memberships,
--     org_memberships) rather than a single polymorphic join. Queries
--     against a known scope are typed at the SQL boundary, and ON DELETE
--     CASCADE works without per-scope filtering.

CREATE TABLE IF NOT EXISTS users (
    id              TEXT PRIMARY KEY,
    email           TEXT NOT NULL,
    email_lower     TEXT NOT NULL,
    email_verified  INTEGER NOT NULL DEFAULT 0,
    created_at_ms   INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS users_email_lower_unique ON users(email_lower);

CREATE TABLE IF NOT EXISTS identities (
    id                TEXT PRIMARY KEY,
    user_id           TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider          TEXT NOT NULL,        -- 'password', 'github', 'google', 'sso:<idp_id>'
    provider_user_id  TEXT NOT NULL,        -- For password identities, this is the user's id.
    secret            TEXT,                 -- argon2id PHC string for password identities; null otherwise.
    created_at_ms     INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS identities_provider_subject_unique
    ON identities(provider, provider_user_id);
CREATE INDEX IF NOT EXISTS identities_user_id ON identities(user_id);

CREATE TABLE IF NOT EXISTS billing_accounts (
    id                  TEXT PRIMARY KEY,
    display_name        TEXT NOT NULL,
    personal            INTEGER NOT NULL DEFAULT 0,
    -- Set when personal=1; references the user the account belongs to.
    owner_user_id       TEXT REFERENCES users(id) ON DELETE CASCADE,
    auto_join_domain    TEXT,
    created_at_ms       INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS billing_accounts_personal_owner
    ON billing_accounts(owner_user_id) WHERE personal = 1;
CREATE INDEX IF NOT EXISTS billing_accounts_auto_join_domain
    ON billing_accounts(auto_join_domain) WHERE auto_join_domain IS NOT NULL;

CREATE TABLE IF NOT EXISTS organizations (
    id                  TEXT PRIMARY KEY,
    display_name        TEXT NOT NULL,
    -- Cascades with the parent billing account; user deletion deletes the
    -- personal billing, which carries the personal org with it.
    billing_account_id  TEXT NOT NULL REFERENCES billing_accounts(id) ON DELETE CASCADE,
    personal            INTEGER NOT NULL DEFAULT 0,
    -- Set when personal=1; references the user the org belongs to.
    owner_user_id       TEXT REFERENCES users(id) ON DELETE CASCADE,
    created_at_ms       INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS organizations_billing_account_id
    ON organizations(billing_account_id);
CREATE UNIQUE INDEX IF NOT EXISTS organizations_personal_owner
    ON organizations(owner_user_id) WHERE personal = 1;

CREATE TABLE IF NOT EXISTS billing_memberships (
    user_id             TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    billing_account_id  TEXT NOT NULL REFERENCES billing_accounts(id) ON DELETE CASCADE,
    role                TEXT NOT NULL,    -- 'owner' | 'member'
    created_at_ms       INTEGER NOT NULL,
    PRIMARY KEY (user_id, billing_account_id)
);
CREATE INDEX IF NOT EXISTS billing_memberships_billing
    ON billing_memberships(billing_account_id);

CREATE TABLE IF NOT EXISTS org_memberships (
    user_id        TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    org_id         TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    role           TEXT NOT NULL,
    created_at_ms  INTEGER NOT NULL,
    PRIMARY KEY (user_id, org_id)
);
CREATE INDEX IF NOT EXISTS org_memberships_org ON org_memberships(org_id);

CREATE TABLE IF NOT EXISTS sso_configs (
    scope_kind           TEXT NOT NULL,    -- 'billing' | 'org'
    scope_id             TEXT NOT NULL,
    idp_id               TEXT NOT NULL,
    kind                 TEXT NOT NULL,    -- 'oidc' | 'saml' (opaque)
    login_url            TEXT NOT NULL,
    break_glass_user_id  TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    required             INTEGER NOT NULL DEFAULT 0,
    updated_at_ms        INTEGER NOT NULL,
    PRIMARY KEY (scope_kind, scope_id)
);
CREATE INDEX IF NOT EXISTS sso_configs_idp ON sso_configs(idp_id);

CREATE TABLE IF NOT EXISTS invitations (
    id                  TEXT PRIMARY KEY,
    scope_kind          TEXT NOT NULL,    -- 'billing' | 'org'
    scope_id            TEXT NOT NULL,
    email               TEXT NOT NULL,
    email_lower         TEXT NOT NULL,
    role                TEXT NOT NULL,
    inviter_user_id     TEXT REFERENCES users(id) ON DELETE SET NULL,
    -- The IdP id frozen at issuance time when the scope was SSO-protected.
    required_idp        TEXT,
    -- The current macaroon nonce. Updated when the invite is re-issued so
    -- previous tokens can no longer be redeemed.
    nonce               TEXT NOT NULL,
    expires_at_ms       INTEGER NOT NULL,
    status              TEXT NOT NULL,    -- 'pending'|'accepted'|'declined'|'revoked'|'expired'
    created_at_ms       INTEGER NOT NULL
);
-- One pending invite per (scope, email).
CREATE UNIQUE INDEX IF NOT EXISTS invitations_scope_email_pending
    ON invitations(scope_kind, scope_id, email_lower)
    WHERE status = 'pending';
CREATE INDEX IF NOT EXISTS invitations_email_lower ON invitations(email_lower);

CREATE TABLE IF NOT EXISTS consumed_nonces (
    nonce         TEXT PRIMARY KEY,
    purpose       TEXT NOT NULL,
    consumed_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS subscriptions (
    billing_account_id    TEXT PRIMARY KEY REFERENCES billing_accounts(id) ON DELETE CASCADE,
    plan                  TEXT NOT NULL DEFAULT 'free',
    status                TEXT NOT NULL DEFAULT 'none', -- 'active'|'past_due'|'canceled'|'none'
    payment_method_token  TEXT,
    updated_at_ms         INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS invoices (
    id                   TEXT PRIMARY KEY,
    billing_account_id   TEXT NOT NULL REFERENCES billing_accounts(id) ON DELETE CASCADE,
    amount_cents         INTEGER NOT NULL,
    currency             TEXT NOT NULL DEFAULT 'USD',
    status               TEXT NOT NULL,                -- 'paid'|'open'|'void'
    issued_at_ms         INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS invoices_billing_account_id ON invoices(billing_account_id);

CREATE TABLE IF NOT EXISTS sso_states (
    state          TEXT PRIMARY KEY,
    scope_hint     TEXT NOT NULL,
    expected_idp   TEXT,
    created_at_ms  INTEGER NOT NULL,
    expires_at_ms  INTEGER NOT NULL
);
