//! D1-backed implementation of [`Repo`].
//!
//! Wasm-only: the JsFuture that backs each D1 call is `!Send` outside wasm,
//! so the futures returned here can't satisfy the `+ Send` bound on `Repo`'s
//! method futures. Native unit tests use [`crate::store::mem::InMemoryRepo`]
//! instead.
//!
//! Multi-row state changes (signup, accept-invitation) go through
//! `D1Database::batch`, which Cloudflare guarantees as atomic
//! (all-or-nothing). Single-row changes use `prepare().run()`.

#![cfg(target_arch = "wasm32")]

use std::collections::HashMap;

use serde::Deserialize;

use worker::send::IntoSendFuture;
use worker::{D1Database, D1PreparedStatement, D1Type};

use crate::domain::{
    BillingAccount, BillingAccountId, BillingMembership, Identity, IdentityId, Invitation,
    InvitationId, InvitationStatus, OrgId, OrgMembership, Organization, Role, ScopeKind,
    ScopeTarget, SsoConfig, SsoState, User, UserId, personal_display_name,
};

use super::error::{StoreError, StoreResult};
use super::repo::{
    BillingAccountWithRole, BillingMemberRow, InvitationAcceptance, NewPasswordUser, NewSsoUser,
    OrgMemberRow, OrgWithRole, Repo,
};

pub struct D1Repo {
    db: D1Database,
}

impl D1Repo {
    pub fn new(db: D1Database) -> Self {
        Self { db }
    }

    async fn query_one<T: for<'de> Deserialize<'de>>(
        &self,
        sql: &str,
        binds: &[D1Type<'_>],
    ) -> StoreResult<Option<T>> {
        self.db
            .prepare(sql)
            .bind_refs(binds)
            .map_err(|e| backend(format!("bind: {e}")))?
            .first(None)
            .into_send()
            .await
            .map_err(|e| backend(format!("query: {e}")))
    }

    async fn query_all<T: for<'de> Deserialize<'de>>(
        &self,
        sql: &str,
        binds: &[D1Type<'_>],
    ) -> StoreResult<Vec<T>> {
        self.db
            .prepare(sql)
            .bind_refs(binds)
            .map_err(|e| backend(format!("bind: {e}")))?
            .all()
            .into_send()
            .await
            .map_err(|e| backend(format!("query: {e}")))?
            .results()
            .map_err(|e| backend(format!("results: {e}")))
    }

    async fn execute(&self, sql: &str, binds: &[D1Type<'_>]) -> StoreResult<()> {
        self.db
            .prepare(sql)
            .bind_refs(binds)
            .map_err(|e| backend(format!("bind: {e}")))?
            .run()
            .into_send()
            .await
            .map_err(|e| backend(format!("execute: {e}")))?;
        Ok(())
    }

    async fn execute_returning_changes(&self, sql: &str, binds: &[D1Type<'_>]) -> StoreResult<u64> {
        let result = self
            .db
            .prepare(sql)
            .bind_refs(binds)
            .map_err(|e| backend(format!("bind: {e}")))?
            .run()
            .into_send()
            .await
            .map_err(|e| backend(format!("execute: {e}")))?;
        let changes = result
            .meta()
            .map_err(|e| backend(format!("meta: {e}")))?
            .and_then(|m| m.changes)
            .unwrap_or(0);
        Ok(changes as u64)
    }

    async fn execute_expecting_change(
        &self,
        sql: &str,
        binds: &[D1Type<'_>],
        not_found_msg: &str,
    ) -> StoreResult<()> {
        if self.execute_returning_changes(sql, binds).await? == 0 {
            return Err(StoreError::not_found(not_found_msg));
        }
        Ok(())
    }

    async fn insert_or_ignore(&self, sql: &str, binds: &[D1Type<'_>]) -> StoreResult<u64> {
        self.execute_returning_changes(sql, binds).await
    }
}

#[derive(Deserialize)]
struct UserRow {
    id: String,
    email: String,
    email_verified: i64,
    created_at_ms: i64,
}
impl From<UserRow> for User {
    fn from(r: UserRow) -> Self {
        User {
            id: UserId::from_string(r.id),
            email: r.email,
            email_verified: r.email_verified != 0,
            created_at_ms: r.created_at_ms,
        }
    }
}

#[derive(Deserialize)]
struct IdentityRow {
    id: String,
    user_id: String,
    provider: String,
    provider_user_id: String,
    secret: Option<String>,
    created_at_ms: i64,
}
impl From<IdentityRow> for Identity {
    fn from(r: IdentityRow) -> Self {
        Identity {
            id: IdentityId::from_string(r.id),
            user_id: UserId::from_string(r.user_id),
            provider: r.provider,
            provider_user_id: r.provider_user_id,
            secret: r.secret,
            created_at_ms: r.created_at_ms,
        }
    }
}

#[derive(Deserialize)]
struct BillingAccountRow {
    id: String,
    display_name: String,
    personal: i64,
    owner_user_id: Option<String>,
    auto_join_domain: Option<String>,
    created_at_ms: i64,
}
impl From<BillingAccountRow> for BillingAccount {
    fn from(r: BillingAccountRow) -> Self {
        BillingAccount {
            id: BillingAccountId::from_string(r.id),
            display_name: r.display_name,
            personal: r.personal != 0,
            owner_user_id: r.owner_user_id.map(UserId::from_string),
            auto_join_domain: r.auto_join_domain,
            created_at_ms: r.created_at_ms,
        }
    }
}

#[derive(Deserialize)]
struct BillingAccountRowWithRole {
    id: String,
    display_name: String,
    personal: i64,
    owner_user_id: Option<String>,
    auto_join_domain: Option<String>,
    created_at_ms: i64,
    role: String,
}

#[derive(Deserialize)]
struct OrgRow {
    id: String,
    display_name: String,
    billing_account_id: String,
    personal: i64,
    owner_user_id: Option<String>,
    created_at_ms: i64,
}
impl From<OrgRow> for Organization {
    fn from(r: OrgRow) -> Self {
        Organization {
            id: OrgId::from_string(r.id),
            display_name: r.display_name,
            billing_account_id: BillingAccountId::from_string(r.billing_account_id),
            personal: r.personal != 0,
            owner_user_id: r.owner_user_id.map(UserId::from_string),
            created_at_ms: r.created_at_ms,
        }
    }
}

#[derive(Deserialize)]
struct OrgRowWithRole {
    id: String,
    display_name: String,
    billing_account_id: String,
    personal: i64,
    owner_user_id: Option<String>,
    created_at_ms: i64,
    role: String,
}

#[derive(Deserialize)]
struct BillingMembershipRow {
    user_id: String,
    billing_account_id: String,
    role: String,
    created_at_ms: i64,
}
impl TryFrom<BillingMembershipRow> for BillingMembership {
    type Error = StoreError;
    fn try_from(r: BillingMembershipRow) -> Result<Self, StoreError> {
        Ok(BillingMembership {
            user_id: UserId::from_string(r.user_id),
            billing_account_id: BillingAccountId::from_string(r.billing_account_id),
            role: r
                .role
                .parse()
                .map_err(|e: String| StoreError::backend(format!("role: {e}")))?,
            created_at_ms: r.created_at_ms,
        })
    }
}

#[derive(Deserialize)]
struct OrgMembershipRow {
    user_id: String,
    org_id: String,
    role: String,
    created_at_ms: i64,
}
impl TryFrom<OrgMembershipRow> for OrgMembership {
    type Error = StoreError;
    fn try_from(r: OrgMembershipRow) -> Result<Self, StoreError> {
        Ok(OrgMembership {
            user_id: UserId::from_string(r.user_id),
            org_id: OrgId::from_string(r.org_id),
            role: r
                .role
                .parse()
                .map_err(|e: String| StoreError::backend(format!("role: {e}")))?,
            created_at_ms: r.created_at_ms,
        })
    }
}

#[derive(Deserialize)]
struct SsoConfigRow {
    scope_kind: String,
    scope_id: String,
    idp_id: String,
    kind: String,
    login_url: String,
    break_glass_user_id: String,
    required: i64,
    updated_at_ms: i64,
}
impl TryFrom<SsoConfigRow> for SsoConfig {
    type Error = StoreError;
    fn try_from(r: SsoConfigRow) -> Result<Self, StoreError> {
        Ok(SsoConfig {
            scope_kind: r
                .scope_kind
                .parse()
                .map_err(|e: String| StoreError::backend(format!("scope_kind: {e}")))?,
            scope_id: r.scope_id,
            idp_id: r.idp_id,
            kind: r.kind,
            login_url: r.login_url,
            break_glass_user_id: UserId::from_string(r.break_glass_user_id),
            required: r.required != 0,
            updated_at_ms: r.updated_at_ms,
        })
    }
}

#[derive(Deserialize)]
struct InvitationRow {
    id: String,
    scope_kind: String,
    scope_id: String,
    email: String,
    role: String,
    inviter_user_id: Option<String>,
    required_idp: Option<String>,
    nonce: String,
    expires_at_ms: i64,
    status: String,
    created_at_ms: i64,
}
impl TryFrom<InvitationRow> for Invitation {
    type Error = StoreError;
    fn try_from(r: InvitationRow) -> Result<Self, StoreError> {
        Ok(Invitation {
            id: InvitationId::from_string(r.id),
            scope_kind: r
                .scope_kind
                .parse()
                .map_err(|e: String| StoreError::backend(format!("scope_kind: {e}")))?,
            scope_id: r.scope_id,
            email: r.email,
            role: r
                .role
                .parse()
                .map_err(|e: String| StoreError::backend(format!("role: {e}")))?,
            inviter_user_id: r.inviter_user_id.map(UserId::from_string),
            required_idp: r.required_idp,
            nonce: r.nonce,
            expires_at_ms: r.expires_at_ms,
            status: r
                .status
                .parse()
                .map_err(|e: String| StoreError::backend(format!("status: {e}")))?,
            created_at_ms: r.created_at_ms,
        })
    }
}

#[derive(Deserialize)]
struct SsoStateRow {
    state: String,
    scope_hint: String,
    expected_idp: Option<String>,
    created_at_ms: i64,
    expires_at_ms: i64,
}
impl From<SsoStateRow> for SsoState {
    fn from(r: SsoStateRow) -> Self {
        SsoState {
            state: r.state,
            scope_hint: r.scope_hint,
            expected_idp: r.expected_idp,
            created_at_ms: r.created_at_ms,
            expires_at_ms: r.expires_at_ms,
        }
    }
}

fn backend(msg: impl Into<String>) -> StoreError {
    StoreError::backend(msg.into())
}

fn ms(x: i64) -> D1Type<'static> {
    D1Type::Real(x as f64)
}

#[derive(Deserialize)]
struct CountRow {
    n: i64,
}

fn placeholders(n: usize) -> String {
    std::iter::repeat_n("?", n).collect::<Vec<_>>().join(",")
}

impl Repo for D1Repo {
    async fn get_user(&self, id: &UserId) -> StoreResult<Option<User>> {
        let row: Option<UserRow> = self
            .query_one(
                "SELECT id, email, email_verified, created_at_ms FROM users WHERE id = ?",
                &[D1Type::Text(id.as_str())],
            )
            .await?;
        Ok(row.map(Into::into))
    }

    async fn get_user_by_email(&self, email: &str) -> StoreResult<Option<User>> {
        let lower = email.to_ascii_lowercase();
        let row: Option<UserRow> = self
            .query_one(
                "SELECT id, email, email_verified, created_at_ms FROM users WHERE email_lower = ?",
                &[D1Type::Text(&lower)],
            )
            .await?;
        Ok(row.map(Into::into))
    }

    async fn mark_email_verified(&self, user_id: &UserId) -> StoreResult<()> {
        self.execute(
            "UPDATE users SET email_verified = 1 WHERE id = ?",
            &[D1Type::Text(user_id.as_str())],
        )
        .await
    }

    async fn list_identities(&self, user_id: &UserId) -> StoreResult<Vec<Identity>> {
        let rows: Vec<IdentityRow> = self
            .query_all(
                "SELECT id, user_id, provider, provider_user_id, secret, created_at_ms \
                 FROM identities WHERE user_id = ? ORDER BY created_at_ms",
                &[D1Type::Text(user_id.as_str())],
            )
            .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn find_identity(
        &self,
        provider: &str,
        provider_user_id: &str,
    ) -> StoreResult<Option<Identity>> {
        let row: Option<IdentityRow> = self
            .query_one(
                "SELECT id, user_id, provider, provider_user_id, secret, created_at_ms \
                 FROM identities WHERE provider = ? AND provider_user_id = ?",
                &[D1Type::Text(provider), D1Type::Text(provider_user_id)],
            )
            .await?;
        Ok(row.map(Into::into))
    }

    async fn add_identity(
        &self,
        user_id: &UserId,
        provider: &str,
        provider_user_id: &str,
        secret: Option<String>,
        now_ms: i64,
    ) -> StoreResult<Identity> {
        let id = IdentityId::new();
        let id_str = id.to_string();
        let secret_arg = secret.as_deref().map_or(D1Type::Null, D1Type::Text);
        let changes = self
            .insert_or_ignore(
                "INSERT OR IGNORE INTO identities (id, user_id, provider, provider_user_id, secret, created_at_ms) \
                 VALUES (?, ?, ?, ?, ?, ?)",
                &[
                    D1Type::Text(&id_str),
                    D1Type::Text(user_id.as_str()),
                    D1Type::Text(provider),
                    D1Type::Text(provider_user_id),
                    secret_arg,
                    ms(now_ms),
                ],
            )
            .await?;
        if changes == 0 {
            return Err(StoreError::already_exists(format!(
                "identity {provider}:{provider_user_id}"
            )));
        }
        Ok(Identity {
            id,
            user_id: user_id.clone(),
            provider: provider.into(),
            provider_user_id: provider_user_id.into(),
            secret,
            created_at_ms: now_ms,
        })
    }

    async fn update_identity_secret(
        &self,
        identity_id: &IdentityId,
        new_secret: String,
    ) -> StoreResult<()> {
        self.execute_expecting_change(
            "UPDATE identities SET secret = ? WHERE id = ?",
            &[
                D1Type::Text(&new_secret),
                D1Type::Text(identity_id.as_str()),
            ],
            &format!("identity {identity_id}"),
        )
        .await
    }

    async fn delete_user(&self, user_id: &UserId) -> StoreResult<()> {
        self.execute_expecting_change(
            "DELETE FROM users WHERE id = ?",
            &[D1Type::Text(user_id.as_str())],
            &format!("user {user_id}"),
        )
        .await
    }

    async fn create_password_user(
        &self,
        input: NewPasswordUser,
        invite: Option<InvitationAcceptance>,
        now_ms: i64,
    ) -> StoreResult<User> {
        let user_id = UserId::new();
        let identity_id = IdentityId::new();
        let billing_id = BillingAccountId::new();
        let org_id = OrgId::new();

        let user_id_str = user_id.to_string();
        let identity_id_str = identity_id.to_string();
        let billing_id_str = billing_id.to_string();
        let org_id_str = org_id.to_string();
        let email = input.email.clone();
        let email_lower = email.to_ascii_lowercase();
        let personal_name = personal_display_name(&email);

        let mut stmts: Vec<D1PreparedStatement> = Vec::new();
        stmts.push(
            self.db
                .prepare(
                    "INSERT INTO users (id, email, email_lower, email_verified, created_at_ms) \
                     VALUES (?, ?, ?, 0, ?)",
                )
                .bind_refs(&[
                    D1Type::Text(&user_id_str),
                    D1Type::Text(&email),
                    D1Type::Text(&email_lower),
                    ms(now_ms),
                ])
                .map_err(|e| backend(format!("bind users: {e}")))?,
        );
        stmts.push(
            self.db
                .prepare(
                    "INSERT INTO identities \
                       (id, user_id, provider, provider_user_id, secret, created_at_ms) \
                     VALUES (?, ?, 'password', ?, ?, ?)",
                )
                .bind_refs(&[
                    D1Type::Text(&identity_id_str),
                    D1Type::Text(&user_id_str),
                    D1Type::Text(&user_id_str),
                    D1Type::Text(&input.password_hash),
                    ms(now_ms),
                ])
                .map_err(|e| backend(format!("bind identities: {e}")))?,
        );
        personal_account_stmts(
            &self.db,
            &mut stmts,
            &user_id_str,
            &billing_id_str,
            &org_id_str,
            &personal_name,
            now_ms,
        )?;

        let invite_payload = invite.as_ref().map(|i| invite_payload(i, now_ms));
        if let Some(payload) = &invite_payload {
            for stmt in payload.statements(&self.db, &user_id_str)? {
                stmts.push(stmt);
            }
        }

        self.db
            .batch(stmts)
            .into_send()
            .await
            .map_err(|e| classify_insert_error("create_password_user", e))?;

        Ok(User {
            id: user_id,
            email: input.email,
            email_verified: false,
            created_at_ms: now_ms,
        })
    }

    async fn create_sso_user(
        &self,
        input: NewSsoUser,
        invite: Option<InvitationAcceptance>,
        auto_join_billing: Option<BillingAccountId>,
        now_ms: i64,
    ) -> StoreResult<User> {
        let user_id = UserId::new();
        let identity_id = IdentityId::new();
        let billing_id = BillingAccountId::new();
        let org_id = OrgId::new();

        let user_id_str = user_id.to_string();
        let identity_id_str = identity_id.to_string();
        let billing_id_str = billing_id.to_string();
        let org_id_str = org_id.to_string();
        let email = input.email.clone();
        let email_lower = email.to_ascii_lowercase();
        let personal_name = personal_display_name(&email);
        let provider = crate::domain::IdentityProvider::sso_str(&input.idp_id);
        let member_role = Role::Member.as_str();
        let auto_join_billing_str = auto_join_billing.as_ref().map(|b| b.to_string());

        let mut stmts: Vec<D1PreparedStatement> = Vec::new();
        stmts.push(
            self.db
                .prepare(
                    "INSERT INTO users (id, email, email_lower, email_verified, created_at_ms) \
                     VALUES (?, ?, ?, 1, ?)",
                )
                .bind_refs(&[
                    D1Type::Text(&user_id_str),
                    D1Type::Text(&email),
                    D1Type::Text(&email_lower),
                    ms(now_ms),
                ])
                .map_err(|e| backend(format!("bind users: {e}")))?,
        );
        stmts.push(
            self.db
                .prepare(
                    "INSERT INTO identities \
                       (id, user_id, provider, provider_user_id, secret, created_at_ms) \
                     VALUES (?, ?, ?, ?, NULL, ?)",
                )
                .bind_refs(&[
                    D1Type::Text(&identity_id_str),
                    D1Type::Text(&user_id_str),
                    D1Type::Text(&provider),
                    D1Type::Text(&input.idp_user_id),
                    ms(now_ms),
                ])
                .map_err(|e| backend(format!("bind identities: {e}")))?,
        );
        personal_account_stmts(
            &self.db,
            &mut stmts,
            &user_id_str,
            &billing_id_str,
            &org_id_str,
            &personal_name,
            now_ms,
        )?;

        if let Some(target) = &auto_join_billing_str {
            stmts.push(
                self.db
                    .prepare(
                        "INSERT INTO billing_memberships (user_id, billing_account_id, role, created_at_ms) \
                         VALUES (?, ?, ?, ?)",
                    )
                    .bind_refs(&[
                        D1Type::Text(&user_id_str),
                        D1Type::Text(target),
                        D1Type::Text(member_role),
                        ms(now_ms),
                    ])
                    .map_err(|e| backend(format!("bind auto_join: {e}")))?,
            );
        }

        let invite_payload = invite.as_ref().map(|i| invite_payload(i, now_ms));
        if let Some(payload) = &invite_payload {
            for stmt in payload.statements(&self.db, &user_id_str)? {
                stmts.push(stmt);
            }
        }

        self.db
            .batch(stmts)
            .into_send()
            .await
            .map_err(|e| classify_insert_error("create_sso_user", e))?;

        Ok(User {
            id: user_id,
            email: input.email,
            email_verified: true,
            created_at_ms: now_ms,
        })
    }

    async fn link_identity(
        &self,
        user_id: &UserId,
        provider: &str,
        provider_user_id: &str,
        secret: Option<String>,
        now_ms: i64,
    ) -> StoreResult<Identity> {
        if let Some(existing) = self.find_identity(provider, provider_user_id).await? {
            if existing.user_id != *user_id {
                return Err(StoreError::conflict(format!(
                    "identity {provider}:{provider_user_id} belongs to another user"
                )));
            }
            return Ok(existing);
        }
        self.add_identity(user_id, provider, provider_user_id, secret, now_ms)
            .await
    }

    async fn get_billing_account(
        &self,
        id: &BillingAccountId,
    ) -> StoreResult<Option<BillingAccount>> {
        let row: Option<BillingAccountRow> = self
            .query_one(
                "SELECT id, display_name, personal, owner_user_id, auto_join_domain, created_at_ms \
                 FROM billing_accounts WHERE id = ?",
                &[D1Type::Text(id.as_str())],
            )
            .await?;
        Ok(row.map(Into::into))
    }

    async fn list_billing_accounts_for_user(
        &self,
        user_id: &UserId,
    ) -> StoreResult<Vec<BillingAccountWithRole>> {
        let rows: Vec<BillingAccountRowWithRole> = self
            .query_all(
                "SELECT b.id, b.display_name, b.personal, b.owner_user_id, b.auto_join_domain, \
                        b.created_at_ms, m.role \
                 FROM billing_accounts b \
                 JOIN billing_memberships m ON m.billing_account_id = b.id \
                 WHERE m.user_id = ? \
                 ORDER BY b.id",
                &[D1Type::Text(user_id.as_str())],
            )
            .await?;
        rows.into_iter()
            .map(|r| {
                let role = r
                    .role
                    .parse::<Role>()
                    .map_err(|e| backend(format!("role: {e}")))?;
                Ok((
                    BillingAccount {
                        id: BillingAccountId::from_string(r.id),
                        display_name: r.display_name,
                        personal: r.personal != 0,
                        owner_user_id: r.owner_user_id.map(UserId::from_string),
                        auto_join_domain: r.auto_join_domain,
                        created_at_ms: r.created_at_ms,
                    },
                    role,
                ))
            })
            .collect()
    }

    async fn create_billing_account(
        &self,
        display_name: String,
        owner_user_id: UserId,
        now_ms: i64,
    ) -> StoreResult<BillingAccount> {
        let id = BillingAccountId::new();
        let id_str = id.to_string();
        let owner_role = Role::Owner.as_str();
        let mut stmts: Vec<D1PreparedStatement> = Vec::new();
        stmts.push(
            self.db
                .prepare(
                    "INSERT INTO billing_accounts \
                       (id, display_name, personal, owner_user_id, auto_join_domain, created_at_ms) \
                     VALUES (?, ?, 0, NULL, NULL, ?)",
                )
                .bind_refs(&[D1Type::Text(&id_str), D1Type::Text(&display_name), ms(now_ms)])
                .map_err(|e| backend(format!("bind billing_accounts: {e}")))?,
        );
        stmts.push(
            self.db
                .prepare(
                    "INSERT INTO billing_memberships (user_id, billing_account_id, role, created_at_ms) \
                     VALUES (?, ?, ?, ?)",
                )
                .bind_refs(&[
                    D1Type::Text(owner_user_id.as_str()),
                    D1Type::Text(&id_str),
                    D1Type::Text(owner_role),
                    ms(now_ms),
                ])
                .map_err(|e| backend(format!("bind billing_memberships: {e}")))?,
        );
        stmts.push(
            self.db
                .prepare(
                    "INSERT INTO subscriptions (billing_account_id, plan, status, payment_method_token, updated_at_ms) \
                     VALUES (?, 'free', 'none', NULL, ?)",
                )
                .bind_refs(&[D1Type::Text(&id_str), ms(now_ms)])
                .map_err(|e| backend(format!("bind subscriptions: {e}")))?,
        );
        self.db
            .batch(stmts)
            .into_send()
            .await
            .map_err(|e| classify_insert_error("create_billing_account", e))?;
        Ok(BillingAccount {
            id,
            display_name,
            personal: false,
            owner_user_id: None,
            auto_join_domain: None,
            created_at_ms: now_ms,
        })
    }

    async fn set_auto_join_domain(
        &self,
        billing_account_id: &BillingAccountId,
        domain: Option<String>,
    ) -> StoreResult<()> {
        let lowered = domain.as_deref().map(str::to_ascii_lowercase);
        let domain_arg = lowered.as_deref().map_or(D1Type::Null, D1Type::Text);
        self.execute_expecting_change(
            "UPDATE billing_accounts SET auto_join_domain = ? WHERE id = ? AND personal = 0",
            &[domain_arg, D1Type::Text(billing_account_id.as_str())],
            &format!("non-personal billing account {billing_account_id}"),
        )
        .await
    }

    async fn find_billing_by_auto_join_domain(
        &self,
        domain: &str,
    ) -> StoreResult<Option<BillingAccount>> {
        let lowered = domain.to_ascii_lowercase();
        let row: Option<BillingAccountRow> = self
            .query_one(
                "SELECT id, display_name, personal, owner_user_id, auto_join_domain, created_at_ms \
                 FROM billing_accounts WHERE personal = 0 AND auto_join_domain = ?",
                &[D1Type::Text(&lowered)],
            )
            .await?;
        Ok(row.map(Into::into))
    }

    async fn get_billing_accounts_by_ids(
        &self,
        ids: &[&str],
    ) -> StoreResult<HashMap<String, BillingAccount>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let placeholders = placeholders(ids.len());
        let sql = format!(
            "SELECT id, display_name, personal, owner_user_id, auto_join_domain, created_at_ms \
             FROM billing_accounts WHERE id IN ({placeholders})"
        );
        let binds: Vec<D1Type<'_>> = ids.iter().map(|s| D1Type::Text(*s)).collect();
        let rows: Vec<BillingAccountRow> = self.query_all(&sql, &binds).await?;
        Ok(rows.into_iter().map(|r| (r.id.clone(), r.into())).collect())
    }

    async fn delete_billing_account(&self, id: &BillingAccountId) -> StoreResult<()> {
        let account = self.get_billing_account(id).await?;
        let Some(account) = account else {
            return Err(StoreError::not_found(format!("billing account {id}")));
        };
        if account.personal {
            return Err(StoreError::conflict(
                "personal billing accounts are deleted with the user",
            ));
        }
        let orgs = self.count_orgs_for_billing(id).await?;
        if orgs > 0 {
            return Err(StoreError::conflict(
                "cannot delete billing account: orgs still attached",
            ));
        }
        self.execute(
            "DELETE FROM billing_accounts WHERE id = ?",
            &[D1Type::Text(id.as_str())],
        )
        .await
    }

    async fn count_orgs_for_billing(
        &self,
        billing_account_id: &BillingAccountId,
    ) -> StoreResult<i64> {
        let row: Option<CountRow> = self
            .query_one(
                "SELECT COUNT(*) AS n FROM organizations WHERE billing_account_id = ?",
                &[D1Type::Text(billing_account_id.as_str())],
            )
            .await?;
        Ok(row.map(|r| r.n).unwrap_or(0))
    }

    async fn count_billing_owners(
        &self,
        billing_account_id: &BillingAccountId,
    ) -> StoreResult<i64> {
        let row: Option<CountRow> = self
            .query_one(
                "SELECT COUNT(*) AS n FROM billing_memberships \
                 WHERE billing_account_id = ? AND role = 'owner'",
                &[D1Type::Text(billing_account_id.as_str())],
            )
            .await?;
        Ok(row.map(|r| r.n).unwrap_or(0))
    }

    async fn count_org_owners(&self, org_id: &OrgId) -> StoreResult<i64> {
        let row: Option<CountRow> = self
            .query_one(
                "SELECT COUNT(*) AS n FROM org_memberships \
                 WHERE org_id = ? AND role = 'owner'",
                &[D1Type::Text(org_id.as_str())],
            )
            .await?;
        Ok(row.map(|r| r.n).unwrap_or(0))
    }

    async fn count_non_personal_owner_memberships(&self, user_id: &UserId) -> StoreResult<i64> {
        let row: Option<CountRow> = self
            .query_one(
                "SELECT COUNT(*) AS n FROM billing_memberships m \
                 JOIN billing_accounts b ON b.id = m.billing_account_id \
                 WHERE m.user_id = ? AND m.role = 'owner' AND b.personal = 0",
                &[D1Type::Text(user_id.as_str())],
            )
            .await?;
        Ok(row.map(|r| r.n).unwrap_or(0))
    }

    async fn create_organization(
        &self,
        display_name: String,
        billing_account_id: BillingAccountId,
        now_ms: i64,
    ) -> StoreResult<Organization> {
        let id = OrgId::new();
        let id_str = id.to_string();
        self.execute(
            "INSERT INTO organizations \
               (id, display_name, billing_account_id, personal, owner_user_id, created_at_ms) \
             VALUES (?, ?, ?, 0, NULL, ?)",
            &[
                D1Type::Text(&id_str),
                D1Type::Text(&display_name),
                D1Type::Text(billing_account_id.as_str()),
                ms(now_ms),
            ],
        )
        .await?;
        Ok(Organization {
            id,
            display_name,
            billing_account_id,
            personal: false,
            owner_user_id: None,
            created_at_ms: now_ms,
        })
    }

    async fn get_organization(&self, id: &OrgId) -> StoreResult<Option<Organization>> {
        let row: Option<OrgRow> = self
            .query_one(
                "SELECT id, display_name, billing_account_id, personal, owner_user_id, created_at_ms \
                 FROM organizations WHERE id = ?",
                &[D1Type::Text(id.as_str())],
            )
            .await?;
        Ok(row.map(Into::into))
    }

    async fn list_organizations_for_billing(
        &self,
        billing_account_id: &BillingAccountId,
    ) -> StoreResult<Vec<Organization>> {
        let rows: Vec<OrgRow> = self
            .query_all(
                "SELECT id, display_name, billing_account_id, personal, owner_user_id, created_at_ms \
                 FROM organizations WHERE billing_account_id = ? ORDER BY id",
                &[D1Type::Text(billing_account_id.as_str())],
            )
            .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_organizations_for_user(&self, user_id: &UserId) -> StoreResult<Vec<OrgWithRole>> {
        let rows: Vec<OrgRowWithRole> = self
            .query_all(
                "SELECT o.id, o.display_name, o.billing_account_id, o.personal, o.owner_user_id, \
                        o.created_at_ms, m.role \
                 FROM organizations o \
                 JOIN org_memberships m ON m.org_id = o.id \
                 WHERE m.user_id = ? \
                 ORDER BY o.id",
                &[D1Type::Text(user_id.as_str())],
            )
            .await?;
        rows.into_iter()
            .map(|r| {
                let role = r
                    .role
                    .parse::<Role>()
                    .map_err(|e| backend(format!("role: {e}")))?;
                Ok((
                    Organization {
                        id: OrgId::from_string(r.id),
                        display_name: r.display_name,
                        billing_account_id: BillingAccountId::from_string(r.billing_account_id),
                        personal: r.personal != 0,
                        owner_user_id: r.owner_user_id.map(UserId::from_string),
                        created_at_ms: r.created_at_ms,
                    },
                    role,
                ))
            })
            .collect()
    }

    async fn get_organizations_by_ids(
        &self,
        ids: &[&str],
    ) -> StoreResult<HashMap<String, Organization>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let placeholders = placeholders(ids.len());
        let sql = format!(
            "SELECT id, display_name, billing_account_id, personal, owner_user_id, created_at_ms \
             FROM organizations WHERE id IN ({placeholders})"
        );
        let binds: Vec<D1Type<'_>> = ids.iter().map(|s| D1Type::Text(*s)).collect();
        let rows: Vec<OrgRow> = self.query_all(&sql, &binds).await?;
        Ok(rows.into_iter().map(|r| (r.id.clone(), r.into())).collect())
    }

    async fn delete_organization(&self, id: &OrgId) -> StoreResult<()> {
        let org = self
            .get_organization(id)
            .await?
            .ok_or_else(|| StoreError::not_found(format!("organization {id}")))?;
        if org.personal {
            return Err(StoreError::conflict(
                "personal organizations are deleted with the user",
            ));
        }
        self.execute(
            "DELETE FROM organizations WHERE id = ?",
            &[D1Type::Text(id.as_str())],
        )
        .await
    }

    async fn add_billing_membership(
        &self,
        user_id: &UserId,
        billing_account_id: &BillingAccountId,
        role: Role,
        now_ms: i64,
    ) -> StoreResult<()> {
        let changes = self
            .insert_or_ignore(
                "INSERT OR IGNORE INTO billing_memberships (user_id, billing_account_id, role, created_at_ms) \
                 VALUES (?, ?, ?, ?)",
                &[
                    D1Type::Text(user_id.as_str()),
                    D1Type::Text(billing_account_id.as_str()),
                    D1Type::Text(role.as_str()),
                    ms(now_ms),
                ],
            )
            .await?;
        if changes == 0 {
            return Err(StoreError::already_exists(format!(
                "billing membership {user_id} -> {billing_account_id}"
            )));
        }
        Ok(())
    }

    async fn get_billing_membership(
        &self,
        user_id: &UserId,
        billing_account_id: &BillingAccountId,
    ) -> StoreResult<Option<BillingMembership>> {
        let row: Option<BillingMembershipRow> = self
            .query_one(
                "SELECT user_id, billing_account_id, role, created_at_ms \
                 FROM billing_memberships WHERE user_id = ? AND billing_account_id = ?",
                &[
                    D1Type::Text(user_id.as_str()),
                    D1Type::Text(billing_account_id.as_str()),
                ],
            )
            .await?;
        match row {
            Some(r) => Ok(Some(r.try_into()?)),
            None => Ok(None),
        }
    }

    async fn update_billing_membership_role(
        &self,
        user_id: &UserId,
        billing_account_id: &BillingAccountId,
        role: Role,
    ) -> StoreResult<()> {
        let existing = self
            .get_billing_membership(user_id, billing_account_id)
            .await?
            .ok_or_else(|| StoreError::not_found("billing membership"))?;
        if existing.role == Role::Owner && role != Role::Owner {
            let owners = self.count_billing_owners(billing_account_id).await?;
            if owners <= 1 {
                return Err(StoreError::conflict(
                    "cannot demote the last owner of a billing account",
                ));
            }
        }
        self.execute(
            "UPDATE billing_memberships SET role = ? WHERE user_id = ? AND billing_account_id = ?",
            &[
                D1Type::Text(role.as_str()),
                D1Type::Text(user_id.as_str()),
                D1Type::Text(billing_account_id.as_str()),
            ],
        )
        .await
    }

    async fn remove_billing_membership(
        &self,
        user_id: &UserId,
        billing_account_id: &BillingAccountId,
    ) -> StoreResult<()> {
        let existing = self
            .get_billing_membership(user_id, billing_account_id)
            .await?
            .ok_or_else(|| StoreError::not_found("billing membership"))?;
        if existing.role == Role::Owner {
            let owners = self.count_billing_owners(billing_account_id).await?;
            if owners <= 1 {
                return Err(StoreError::conflict(
                    "cannot remove the last owner of a billing account",
                ));
            }
        }
        self.execute(
            "DELETE FROM billing_memberships WHERE user_id = ? AND billing_account_id = ?",
            &[
                D1Type::Text(user_id.as_str()),
                D1Type::Text(billing_account_id.as_str()),
            ],
        )
        .await
    }

    async fn list_billing_memberships(
        &self,
        billing_account_id: &BillingAccountId,
    ) -> StoreResult<Vec<BillingMemberRow>> {
        #[derive(Deserialize)]
        struct Row {
            user_id: String,
            billing_account_id: String,
            role: String,
            membership_created_at_ms: i64,
            email: String,
            email_verified: i64,
            user_created_at_ms: i64,
        }
        let rows: Vec<Row> = self
            .query_all(
                "SELECT m.user_id, m.billing_account_id, m.role, m.created_at_ms AS membership_created_at_ms, \
                        u.email, u.email_verified, u.created_at_ms AS user_created_at_ms \
                 FROM billing_memberships m \
                 JOIN users u ON u.id = m.user_id \
                 WHERE m.billing_account_id = ? \
                 ORDER BY m.user_id",
                &[D1Type::Text(billing_account_id.as_str())],
            )
            .await?;
        rows.into_iter()
            .map(|r| {
                let role = r
                    .role
                    .parse::<Role>()
                    .map_err(|e| backend(format!("role: {e}")))?;
                Ok((
                    BillingMembership {
                        user_id: UserId::from_string(r.user_id.clone()),
                        billing_account_id: BillingAccountId::from_string(r.billing_account_id),
                        role,
                        created_at_ms: r.membership_created_at_ms,
                    },
                    User {
                        id: UserId::from_string(r.user_id),
                        email: r.email,
                        email_verified: r.email_verified != 0,
                        created_at_ms: r.user_created_at_ms,
                    },
                ))
            })
            .collect()
    }

    async fn add_org_membership(
        &self,
        user_id: &UserId,
        org_id: &OrgId,
        role: Role,
        now_ms: i64,
    ) -> StoreResult<()> {
        let changes = self
            .insert_or_ignore(
                "INSERT OR IGNORE INTO org_memberships (user_id, org_id, role, created_at_ms) \
                 VALUES (?, ?, ?, ?)",
                &[
                    D1Type::Text(user_id.as_str()),
                    D1Type::Text(org_id.as_str()),
                    D1Type::Text(role.as_str()),
                    ms(now_ms),
                ],
            )
            .await?;
        if changes == 0 {
            return Err(StoreError::already_exists(format!(
                "org membership {user_id} -> {org_id}"
            )));
        }
        Ok(())
    }

    async fn get_org_membership(
        &self,
        user_id: &UserId,
        org_id: &OrgId,
    ) -> StoreResult<Option<OrgMembership>> {
        let row: Option<OrgMembershipRow> = self
            .query_one(
                "SELECT user_id, org_id, role, created_at_ms FROM org_memberships \
                 WHERE user_id = ? AND org_id = ?",
                &[
                    D1Type::Text(user_id.as_str()),
                    D1Type::Text(org_id.as_str()),
                ],
            )
            .await?;
        match row {
            Some(r) => Ok(Some(r.try_into()?)),
            None => Ok(None),
        }
    }

    async fn update_org_membership_role(
        &self,
        user_id: &UserId,
        org_id: &OrgId,
        role: Role,
    ) -> StoreResult<()> {
        let existing = self
            .get_org_membership(user_id, org_id)
            .await?
            .ok_or_else(|| StoreError::not_found("org membership"))?;
        if existing.role == Role::Owner && role != Role::Owner {
            if self.count_org_owners(org_id).await? <= 1 {
                return Err(StoreError::conflict(
                    "cannot demote the last owner of an organization",
                ));
            }
        }
        self.execute(
            "UPDATE org_memberships SET role = ? WHERE user_id = ? AND org_id = ?",
            &[
                D1Type::Text(role.as_str()),
                D1Type::Text(user_id.as_str()),
                D1Type::Text(org_id.as_str()),
            ],
        )
        .await
    }

    async fn remove_org_membership(&self, user_id: &UserId, org_id: &OrgId) -> StoreResult<()> {
        let existing = self
            .get_org_membership(user_id, org_id)
            .await?
            .ok_or_else(|| StoreError::not_found("org membership"))?;
        if existing.role == Role::Owner {
            if self.count_org_owners(org_id).await? <= 1 {
                return Err(StoreError::conflict(
                    "cannot remove the last owner of an organization",
                ));
            }
        }
        self.execute(
            "DELETE FROM org_memberships WHERE user_id = ? AND org_id = ?",
            &[
                D1Type::Text(user_id.as_str()),
                D1Type::Text(org_id.as_str()),
            ],
        )
        .await
    }

    async fn list_org_memberships(&self, org_id: &OrgId) -> StoreResult<Vec<OrgMemberRow>> {
        #[derive(Deserialize)]
        struct Row {
            user_id: String,
            org_id: String,
            role: String,
            membership_created_at_ms: i64,
            email: String,
            email_verified: i64,
            user_created_at_ms: i64,
        }
        let rows: Vec<Row> = self
            .query_all(
                "SELECT m.user_id, m.org_id, m.role, m.created_at_ms AS membership_created_at_ms, \
                        u.email, u.email_verified, u.created_at_ms AS user_created_at_ms \
                 FROM org_memberships m \
                 JOIN users u ON u.id = m.user_id \
                 WHERE m.org_id = ? \
                 ORDER BY m.user_id",
                &[D1Type::Text(org_id.as_str())],
            )
            .await?;
        rows.into_iter()
            .map(|r| {
                let role = r
                    .role
                    .parse::<Role>()
                    .map_err(|e| backend(format!("role: {e}")))?;
                Ok((
                    OrgMembership {
                        user_id: UserId::from_string(r.user_id.clone()),
                        org_id: OrgId::from_string(r.org_id),
                        role,
                        created_at_ms: r.membership_created_at_ms,
                    },
                    User {
                        id: UserId::from_string(r.user_id),
                        email: r.email,
                        email_verified: r.email_verified != 0,
                        created_at_ms: r.user_created_at_ms,
                    },
                ))
            })
            .collect()
    }

    async fn list_sso_configs_by_scope_ids(
        &self,
        scope_kind: ScopeKind,
        scope_ids: &[&str],
    ) -> StoreResult<HashMap<String, SsoConfig>> {
        if scope_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let placeholders = placeholders(scope_ids.len());
        let sql = format!(
            "SELECT scope_kind, scope_id, idp_id, kind, login_url, break_glass_user_id, required, updated_at_ms \
             FROM sso_configs WHERE scope_kind = ? AND scope_id IN ({placeholders})"
        );
        let mut binds: Vec<D1Type<'_>> = Vec::with_capacity(scope_ids.len() + 1);
        binds.push(D1Type::Text(scope_kind.as_str()));
        binds.extend(scope_ids.iter().map(|s| D1Type::Text(*s)));
        let rows: Vec<SsoConfigRow> = self.query_all(&sql, &binds).await?;
        rows.into_iter()
            .map(|r| {
                let scope_id = r.scope_id.clone();
                let cfg: SsoConfig = r.try_into()?;
                Ok((scope_id, cfg))
            })
            .collect()
    }

    async fn get_sso_config(
        &self,
        scope_kind: ScopeKind,
        scope_id: &str,
    ) -> StoreResult<Option<SsoConfig>> {
        let row: Option<SsoConfigRow> = self
            .query_one(
                "SELECT scope_kind, scope_id, idp_id, kind, login_url, break_glass_user_id, required, updated_at_ms \
                 FROM sso_configs WHERE scope_kind = ? AND scope_id = ?",
                &[D1Type::Text(scope_kind.as_str()), D1Type::Text(scope_id)],
            )
            .await?;
        match row {
            Some(r) => Ok(Some(r.try_into()?)),
            None => Ok(None),
        }
    }

    async fn upsert_sso_config(&self, config: SsoConfig) -> StoreResult<()> {
        let required: i64 = if config.required { 1 } else { 0 };
        self.execute(
            "INSERT INTO sso_configs (scope_kind, scope_id, idp_id, kind, login_url, break_glass_user_id, required, updated_at_ms) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(scope_kind, scope_id) DO UPDATE SET \
               idp_id = excluded.idp_id, \
               kind = excluded.kind, \
               login_url = excluded.login_url, \
               break_glass_user_id = excluded.break_glass_user_id, \
               required = excluded.required, \
               updated_at_ms = excluded.updated_at_ms",
            &[
                D1Type::Text(config.scope_kind.as_str()),
                D1Type::Text(&config.scope_id),
                D1Type::Text(&config.idp_id),
                D1Type::Text(&config.kind),
                D1Type::Text(&config.login_url),
                D1Type::Text(config.break_glass_user_id.as_str()),
                D1Type::Integer(required as i32),
                ms(config.updated_at_ms),
            ],
        )
        .await
    }

    async fn delete_sso_config(&self, scope_kind: ScopeKind, scope_id: &str) -> StoreResult<()> {
        self.execute(
            "DELETE FROM sso_configs WHERE scope_kind = ? AND scope_id = ?",
            &[D1Type::Text(scope_kind.as_str()), D1Type::Text(scope_id)],
        )
        .await
    }

    async fn create_invitation(&self, invitation: Invitation) -> StoreResult<()> {
        let inviter = invitation.inviter_user_id.as_ref().map(|u| u.to_string());
        let inviter_arg = inviter.as_deref().map_or(D1Type::Null, D1Type::Text);
        let required_idp_arg = invitation
            .required_idp
            .as_deref()
            .map_or(D1Type::Null, D1Type::Text);
        let email_lower = invitation.email.to_ascii_lowercase();

        let changes = self
            .insert_or_ignore(
                "INSERT OR IGNORE INTO invitations \
                   (id, scope_kind, scope_id, email, email_lower, role, inviter_user_id, required_idp, nonce, expires_at_ms, status, created_at_ms) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                &[
                    D1Type::Text(invitation.id.as_str()),
                    D1Type::Text(invitation.scope_kind.as_str()),
                    D1Type::Text(&invitation.scope_id),
                    D1Type::Text(&invitation.email),
                    D1Type::Text(&email_lower),
                    D1Type::Text(invitation.role.as_str()),
                    inviter_arg,
                    required_idp_arg,
                    D1Type::Text(&invitation.nonce),
                    ms(invitation.expires_at_ms),
                    D1Type::Text(invitation.status.as_str()),
                    ms(invitation.created_at_ms),
                ],
            )
            .await?;
        if changes == 0 {
            return Err(StoreError::already_exists(format!(
                "pending invite for {} on {}:{}",
                invitation.email,
                invitation.scope_kind.as_str(),
                invitation.scope_id
            )));
        }
        Ok(())
    }

    async fn get_invitation(&self, id: &InvitationId) -> StoreResult<Option<Invitation>> {
        let row: Option<InvitationRow> = self
            .query_one(
                "SELECT id, scope_kind, scope_id, email, role, inviter_user_id, required_idp, nonce, expires_at_ms, status, created_at_ms \
                 FROM invitations WHERE id = ?",
                &[D1Type::Text(id.as_str())],
            )
            .await?;
        match row {
            Some(r) => Ok(Some(r.try_into()?)),
            None => Ok(None),
        }
    }

    async fn list_pending_invitations_by_email(&self, email: &str) -> StoreResult<Vec<Invitation>> {
        let lower = email.to_ascii_lowercase();
        let rows: Vec<InvitationRow> = self
            .query_all(
                "SELECT id, scope_kind, scope_id, email, role, inviter_user_id, required_idp, nonce, expires_at_ms, status, created_at_ms \
                 FROM invitations WHERE email_lower = ? AND status = 'pending' ORDER BY created_at_ms",
                &[D1Type::Text(&lower)],
            )
            .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    async fn list_pending_invitation_by_scope_email(
        &self,
        scope_kind: ScopeKind,
        scope_id: &str,
        email: &str,
    ) -> StoreResult<Option<Invitation>> {
        let lower = email.to_ascii_lowercase();
        let row: Option<InvitationRow> = self
            .query_one(
                "SELECT id, scope_kind, scope_id, email, role, inviter_user_id, required_idp, nonce, expires_at_ms, status, created_at_ms \
                 FROM invitations WHERE scope_kind = ? AND scope_id = ? AND email_lower = ? AND status = 'pending'",
                &[
                    D1Type::Text(scope_kind.as_str()),
                    D1Type::Text(scope_id),
                    D1Type::Text(&lower),
                ],
            )
            .await?;
        match row {
            Some(r) => Ok(Some(r.try_into()?)),
            None => Ok(None),
        }
    }

    async fn update_invitation_status(
        &self,
        id: &InvitationId,
        status: InvitationStatus,
    ) -> StoreResult<()> {
        self.execute(
            "UPDATE invitations SET status = ? WHERE id = ?",
            &[D1Type::Text(status.as_str()), D1Type::Text(id.as_str())],
        )
        .await
    }

    async fn refresh_invitation_token(
        &self,
        id: &InvitationId,
        new_nonce: String,
        new_expiry_ms: i64,
    ) -> StoreResult<()> {
        self.execute(
            "UPDATE invitations SET nonce = ?, expires_at_ms = ?, status = 'pending' WHERE id = ?",
            &[
                D1Type::Text(&new_nonce),
                ms(new_expiry_ms),
                D1Type::Text(id.as_str()),
            ],
        )
        .await
    }

    async fn accept_invitation_existing_user(
        &self,
        user_id: &UserId,
        acceptance: InvitationAcceptance,
        nonce: &str,
        now_ms: i64,
    ) -> StoreResult<()> {
        let user_id_str = user_id.to_string();
        let invitation_id_str = acceptance.invitation_id.to_string();

        let mut stmts: Vec<D1PreparedStatement> = Vec::new();
        // Stamp the nonce first; if this would be a duplicate the batch
        // fails (atomicity rolls everything back).
        stmts.push(
            self.db
                .prepare(
                    "INSERT INTO consumed_nonces (nonce, purpose, consumed_at_ms) VALUES (?, 'invitation', ?)",
                )
                .bind_refs(&[D1Type::Text(nonce), ms(now_ms)])
                .map_err(|e| backend(format!("bind nonce: {e}")))?,
        );
        match &acceptance.target {
            ScopeTarget::Billing(billing_id) => stmts.push(
                self.db
                    .prepare(
                        "INSERT INTO billing_memberships (user_id, billing_account_id, role, created_at_ms) \
                         VALUES (?, ?, ?, ?)",
                    )
                    .bind_refs(&[
                        D1Type::Text(&user_id_str),
                        D1Type::Text(billing_id.as_str()),
                        D1Type::Text(acceptance.role.as_str()),
                        ms(now_ms),
                    ])
                    .map_err(|e| backend(format!("bind billing membership: {e}")))?,
            ),
            ScopeTarget::Org(org_id) => stmts.push(
                self.db
                    .prepare(
                        "INSERT INTO org_memberships (user_id, org_id, role, created_at_ms) \
                         VALUES (?, ?, ?, ?)",
                    )
                    .bind_refs(&[
                        D1Type::Text(&user_id_str),
                        D1Type::Text(org_id.as_str()),
                        D1Type::Text(acceptance.role.as_str()),
                        ms(now_ms),
                    ])
                    .map_err(|e| backend(format!("bind org membership: {e}")))?,
            ),
        }
        stmts.push(
            self.db
                .prepare("UPDATE invitations SET status = 'accepted' WHERE id = ?")
                .bind_refs(&[D1Type::Text(&invitation_id_str)])
                .map_err(|e| backend(format!("bind invitation update: {e}")))?,
        );

        self.db
            .batch(stmts)
            .into_send()
            .await
            .map_err(|e| classify_insert_error("accept_invitation_existing_user", e))?;
        Ok(())
    }

    async fn consume_nonce(&self, nonce: &str, purpose: &str, now_ms: i64) -> StoreResult<()> {
        let changes = self
            .insert_or_ignore(
                "INSERT OR IGNORE INTO consumed_nonces (nonce, purpose, consumed_at_ms) VALUES (?, ?, ?)",
                &[D1Type::Text(nonce), D1Type::Text(purpose), ms(now_ms)],
            )
            .await?;
        if changes == 0 {
            return Err(StoreError::already_exists(format!("nonce {nonce}")));
        }
        Ok(())
    }

    async fn store_sso_state(&self, state: SsoState) -> StoreResult<()> {
        let expected_idp_arg = state
            .expected_idp
            .as_deref()
            .map_or(D1Type::Null, D1Type::Text);
        self.execute(
            "INSERT INTO sso_states (state, scope_hint, expected_idp, created_at_ms, expires_at_ms) \
             VALUES (?, ?, ?, ?, ?)",
            &[
                D1Type::Text(&state.state),
                D1Type::Text(&state.scope_hint),
                expected_idp_arg,
                ms(state.created_at_ms),
                ms(state.expires_at_ms),
            ],
        )
        .await
    }

    async fn consume_sso_state(&self, state: &str, now_ms: i64) -> StoreResult<Option<SsoState>> {
        let row: Option<SsoStateRow> = self
            .query_one(
                "DELETE FROM sso_states WHERE state = ? \
                 RETURNING state, scope_hint, expected_idp, created_at_ms, expires_at_ms",
                &[D1Type::Text(state)],
            )
            .await?;
        let Some(row) = row else { return Ok(None) };
        if row.expires_at_ms < now_ms {
            return Ok(None);
        }
        Ok(Some(row.into()))
    }
}

/// Classify a D1 batch error as `AlreadyExists` when the error string
/// contains a SQLite constraint-violation marker. Only used for batch
/// operations (`db.batch()`); single-row inserts use `INSERT OR IGNORE`
/// with an explicit changes-count check instead.
fn classify_insert_error(context: &'static str, err: worker::Error) -> StoreError {
    let s = err.to_string();
    let lower = s.to_lowercase();
    if lower.contains("unique constraint failed")
        || lower.contains("primary key constraint failed")
        || lower.contains("sqlite_constraint")
    {
        StoreError::AlreadyExists(context.to_owned())
    } else {
        StoreError::Backend(context.to_owned())
    }
}

/// Builds the personal billing account + subscription + personal org +
/// owner memberships that every new user gets. Shared between the
/// password and SSO signup paths.
fn personal_account_stmts(
    db: &D1Database,
    stmts: &mut Vec<D1PreparedStatement>,
    user_id_str: &str,
    billing_id_str: &str,
    org_id_str: &str,
    personal_name: &str,
    now_ms: i64,
) -> StoreResult<()> {
    let owner_role = Role::Owner.as_str();
    stmts.push(
        db.prepare(
            "INSERT INTO billing_accounts \
               (id, display_name, personal, owner_user_id, auto_join_domain, created_at_ms) \
             VALUES (?, ?, 1, ?, NULL, ?)",
        )
        .bind_refs(&[
            D1Type::Text(billing_id_str),
            D1Type::Text(personal_name),
            D1Type::Text(user_id_str),
            ms(now_ms),
        ])
        .map_err(|e| backend(format!("bind billing_accounts: {e}")))?,
    );
    stmts.push(
        db.prepare(
            "INSERT INTO billing_memberships (user_id, billing_account_id, role, created_at_ms) \
             VALUES (?, ?, ?, ?)",
        )
        .bind_refs(&[
            D1Type::Text(user_id_str),
            D1Type::Text(billing_id_str),
            D1Type::Text(owner_role),
            ms(now_ms),
        ])
        .map_err(|e| backend(format!("bind billing_memberships: {e}")))?,
    );
    stmts.push(
        db.prepare(
            "INSERT INTO subscriptions (billing_account_id, plan, status, payment_method_token, updated_at_ms) \
             VALUES (?, 'free', 'none', NULL, ?)",
        )
        .bind_refs(&[D1Type::Text(billing_id_str), ms(now_ms)])
        .map_err(|e| backend(format!("bind subscriptions: {e}")))?,
    );
    stmts.push(
        db.prepare(
            "INSERT INTO organizations \
               (id, display_name, billing_account_id, personal, owner_user_id, created_at_ms) \
             VALUES (?, ?, ?, 1, ?, ?)",
        )
        .bind_refs(&[
            D1Type::Text(org_id_str),
            D1Type::Text(personal_name),
            D1Type::Text(billing_id_str),
            D1Type::Text(user_id_str),
            ms(now_ms),
        ])
        .map_err(|e| backend(format!("bind organizations: {e}")))?,
    );
    stmts.push(
        db.prepare(
            "INSERT INTO org_memberships (user_id, org_id, role, created_at_ms) \
             VALUES (?, ?, ?, ?)",
        )
        .bind_refs(&[
            D1Type::Text(user_id_str),
            D1Type::Text(org_id_str),
            D1Type::Text(owner_role),
            ms(now_ms),
        ])
        .map_err(|e| backend(format!("bind org_memberships: {e}")))?,
    );
    Ok(())
}

/// Holds the bound strings needed to assemble the invitation-acceptance
/// statements, separated so the borrow checker can see they outlive the
/// statement vector that references them.
struct InvitePayload {
    user_invitation_id: String,
    role: &'static str,
    target: ScopeTarget,
    nonce: String,
    now_ms: i64,
}

fn invite_payload(invite: &InvitationAcceptance, now_ms: i64) -> InvitePayload {
    InvitePayload {
        user_invitation_id: invite.invitation_id.to_string(),
        role: invite.role.as_str(),
        target: invite.target.clone(),
        nonce: format!("invite-accept:{}", invite.invitation_id),
        now_ms,
    }
}

impl InvitePayload {
    fn statements(
        &self,
        db: &D1Database,
        user_id_str: &str,
    ) -> StoreResult<Vec<D1PreparedStatement>> {
        let mut out = Vec::new();
        out.push(
            db.prepare(
                "INSERT INTO consumed_nonces (nonce, purpose, consumed_at_ms) VALUES (?, 'invitation', ?)",
            )
            .bind_refs(&[D1Type::Text(&self.nonce), ms(self.now_ms)])
            .map_err(|e| backend(format!("bind nonce: {e}")))?,
        );
        match &self.target {
            ScopeTarget::Billing(billing_id) => out.push(
                db.prepare(
                    "INSERT INTO billing_memberships (user_id, billing_account_id, role, created_at_ms) \
                     VALUES (?, ?, ?, ?)",
                )
                .bind_refs(&[
                    D1Type::Text(user_id_str),
                    D1Type::Text(billing_id.as_str()),
                    D1Type::Text(self.role),
                    ms(self.now_ms),
                ])
                .map_err(|e| backend(format!("bind invite billing: {e}")))?,
            ),
            ScopeTarget::Org(org_id) => out.push(
                db.prepare(
                    "INSERT INTO org_memberships (user_id, org_id, role, created_at_ms) \
                     VALUES (?, ?, ?, ?)",
                )
                .bind_refs(&[
                    D1Type::Text(user_id_str),
                    D1Type::Text(org_id.as_str()),
                    D1Type::Text(self.role),
                    ms(self.now_ms),
                ])
                .map_err(|e| backend(format!("bind invite org: {e}")))?,
            ),
        }
        out.push(
            db.prepare("UPDATE invitations SET status = 'accepted' WHERE id = ?")
                .bind_refs(&[D1Type::Text(&self.user_invitation_id)])
                .map_err(|e| backend(format!("bind invite update: {e}")))?,
        );
        Ok(out)
    }
}
