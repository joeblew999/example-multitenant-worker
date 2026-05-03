//! The single `Repo` trait that backs every domain operation.
//!
//! High-level "do everything" methods exist for the multi-row state changes
//! that need to be atomic in production — `create_password_user` bundles
//! user + identity + personal billing + owner membership creation, etc. The
//! D1 implementation maps these to `D1Database::batch` (atomic per
//! Cloudflare's docs); the in-memory implementation runs them under a
//! single mutex.

use std::collections::HashMap;
use std::future::Future;

use crate::domain::{
    BillingAccount, BillingAccountId, BillingMembership, Identity, IdentityId, Invitation,
    InvitationId, InvitationStatus, OrgId, OrgMembership, Organization, Role, ScopeKind,
    ScopeTarget, SsoConfig, SsoState, User, UserId,
};

use super::error::StoreResult;

/// Input bundle for SSO-driven user creation. Combined into a single trait
/// method so the user, identity, personal billing, and owner membership
/// creation happen as one atomic unit at the storage boundary.
#[derive(Clone, Debug)]
pub struct NewSsoUser {
    pub email: String,
    pub idp_id: String,
    pub idp_user_id: String,
}

/// Input bundle for password signup.
#[derive(Clone, Debug)]
pub struct NewPasswordUser {
    pub email: String,
    pub password_hash: String,
}

/// Resolved invitation passed into the "signup-and-accept" / "accept as
/// existing user" entry points.
#[derive(Clone, Debug)]
pub struct InvitationAcceptance {
    pub invitation_id: InvitationId,
    pub target: ScopeTarget,
    pub role: Role,
}

/// `(BillingAccount, role)` pair returned for the "list billing accounts I'm
/// in" query.
pub type BillingAccountWithRole = (BillingAccount, Role);
pub type OrgWithRole = (Organization, Role);
pub type BillingMemberRow = (BillingMembership, User);
pub type OrgMemberRow = (OrgMembership, User);

pub trait Repo: Send + Sync + 'static {
    // ----- Users / identities -----

    fn get_user(&self, id: &UserId) -> impl Future<Output = StoreResult<Option<User>>> + Send;

    fn get_user_by_email(
        &self,
        email: &str,
    ) -> impl Future<Output = StoreResult<Option<User>>> + Send;

    fn mark_email_verified(&self, user_id: &UserId)
    -> impl Future<Output = StoreResult<()>> + Send;

    fn list_identities(
        &self,
        user_id: &UserId,
    ) -> impl Future<Output = StoreResult<Vec<Identity>>> + Send;

    fn find_identity(
        &self,
        provider: &str,
        provider_user_id: &str,
    ) -> impl Future<Output = StoreResult<Option<Identity>>> + Send;

    fn add_identity(
        &self,
        user_id: &UserId,
        provider: &str,
        provider_user_id: &str,
        secret: Option<String>,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<Identity>> + Send;

    fn update_identity_secret(
        &self,
        identity_id: &IdentityId,
        new_secret: String,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    fn delete_user(&self, user_id: &UserId) -> impl Future<Output = StoreResult<()>> + Send;

    // ----- High-level signup paths (atomic at the storage boundary) -----

    fn create_password_user(
        &self,
        input: NewPasswordUser,
        invite: Option<InvitationAcceptance>,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<User>> + Send;

    fn create_sso_user(
        &self,
        input: NewSsoUser,
        invite: Option<InvitationAcceptance>,
        auto_join_billing: Option<BillingAccountId>,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<User>> + Send;

    /// Used by `LinkIdentity` and the JIT path where an SSO assertion comes
    /// in for a known user. Returns the existing identity if one already
    /// matched `(provider, provider_user_id)`.
    fn link_identity(
        &self,
        user_id: &UserId,
        provider: &str,
        provider_user_id: &str,
        secret: Option<String>,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<Identity>> + Send;

    // ----- Billing accounts -----

    fn get_billing_account(
        &self,
        id: &BillingAccountId,
    ) -> impl Future<Output = StoreResult<Option<BillingAccount>>> + Send;

    fn list_billing_accounts_for_user(
        &self,
        user_id: &UserId,
    ) -> impl Future<Output = StoreResult<Vec<BillingAccountWithRole>>> + Send;

    fn create_billing_account(
        &self,
        display_name: String,
        owner_user_id: UserId,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<BillingAccount>> + Send;

    fn set_auto_join_domain(
        &self,
        billing_account_id: &BillingAccountId,
        domain: Option<String>,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    fn find_billing_by_auto_join_domain(
        &self,
        domain: &str,
    ) -> impl Future<Output = StoreResult<Option<BillingAccount>>> + Send;

    fn delete_billing_account(
        &self,
        id: &BillingAccountId,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    /// Batch fetch billing accounts by id. Missing ids are absent from the
    /// returned map. Used by invitation list endpoints to resolve scope
    /// display names without an N+1.
    fn get_billing_accounts_by_ids(
        &self,
        ids: &[&str],
    ) -> impl Future<Output = StoreResult<HashMap<String, BillingAccount>>> + Send;

    fn count_orgs_for_billing(
        &self,
        billing_account_id: &BillingAccountId,
    ) -> impl Future<Output = StoreResult<i64>> + Send;

    fn count_billing_owners(
        &self,
        billing_account_id: &BillingAccountId,
    ) -> impl Future<Output = StoreResult<i64>> + Send;

    fn count_org_owners(&self, org_id: &OrgId) -> impl Future<Output = StoreResult<i64>> + Send;

    fn count_non_personal_owner_memberships(
        &self,
        user_id: &UserId,
    ) -> impl Future<Output = StoreResult<i64>> + Send;

    // ----- Organizations -----

    fn create_organization(
        &self,
        display_name: String,
        billing_account_id: BillingAccountId,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<Organization>> + Send;

    fn get_organization(
        &self,
        id: &OrgId,
    ) -> impl Future<Output = StoreResult<Option<Organization>>> + Send;

    fn list_organizations_for_billing(
        &self,
        billing_account_id: &BillingAccountId,
    ) -> impl Future<Output = StoreResult<Vec<Organization>>> + Send;

    fn list_organizations_for_user(
        &self,
        user_id: &UserId,
    ) -> impl Future<Output = StoreResult<Vec<OrgWithRole>>> + Send;

    fn delete_organization(&self, id: &OrgId) -> impl Future<Output = StoreResult<()>> + Send;

    /// Batch fetch organizations by id. Missing ids are absent from the
    /// returned map. Used alongside [`get_billing_accounts_by_ids`] for
    /// invitation list endpoints.
    fn get_organizations_by_ids(
        &self,
        ids: &[&str],
    ) -> impl Future<Output = StoreResult<HashMap<String, Organization>>> + Send;

    // ----- Memberships -----

    fn add_billing_membership(
        &self,
        user_id: &UserId,
        billing_account_id: &BillingAccountId,
        role: Role,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    fn get_billing_membership(
        &self,
        user_id: &UserId,
        billing_account_id: &BillingAccountId,
    ) -> impl Future<Output = StoreResult<Option<BillingMembership>>> + Send;

    fn update_billing_membership_role(
        &self,
        user_id: &UserId,
        billing_account_id: &BillingAccountId,
        role: Role,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    fn remove_billing_membership(
        &self,
        user_id: &UserId,
        billing_account_id: &BillingAccountId,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    fn list_billing_memberships(
        &self,
        billing_account_id: &BillingAccountId,
    ) -> impl Future<Output = StoreResult<Vec<BillingMemberRow>>> + Send;

    fn add_org_membership(
        &self,
        user_id: &UserId,
        org_id: &OrgId,
        role: Role,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    fn get_org_membership(
        &self,
        user_id: &UserId,
        org_id: &OrgId,
    ) -> impl Future<Output = StoreResult<Option<OrgMembership>>> + Send;

    fn update_org_membership_role(
        &self,
        user_id: &UserId,
        org_id: &OrgId,
        role: Role,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    fn remove_org_membership(
        &self,
        user_id: &UserId,
        org_id: &OrgId,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    fn list_org_memberships(
        &self,
        org_id: &OrgId,
    ) -> impl Future<Output = StoreResult<Vec<OrgMemberRow>>> + Send;

    // ----- SSO config -----

    fn get_sso_config(
        &self,
        scope_kind: ScopeKind,
        scope_id: &str,
    ) -> impl Future<Output = StoreResult<Option<SsoConfig>>> + Send;

    /// Batch fetch SSO configs for many scope ids of the same `scope_kind`.
    /// Used by list endpoints (org / billing) to avoid an N+1 lookup. Scope
    /// ids without a config are simply absent from the returned map.
    fn list_sso_configs_by_scope_ids(
        &self,
        scope_kind: ScopeKind,
        scope_ids: &[&str],
    ) -> impl Future<Output = StoreResult<HashMap<String, SsoConfig>>> + Send;

    fn upsert_sso_config(&self, config: SsoConfig) -> impl Future<Output = StoreResult<()>> + Send;

    fn delete_sso_config(
        &self,
        scope_kind: ScopeKind,
        scope_id: &str,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    // ----- Invitations -----

    fn create_invitation(
        &self,
        invitation: Invitation,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    fn get_invitation(
        &self,
        id: &InvitationId,
    ) -> impl Future<Output = StoreResult<Option<Invitation>>> + Send;

    fn list_pending_invitations_by_email(
        &self,
        email: &str,
    ) -> impl Future<Output = StoreResult<Vec<Invitation>>> + Send;

    fn list_pending_invitation_by_scope_email(
        &self,
        scope_kind: ScopeKind,
        scope_id: &str,
        email: &str,
    ) -> impl Future<Output = StoreResult<Option<Invitation>>> + Send;

    fn update_invitation_status(
        &self,
        id: &InvitationId,
        status: InvitationStatus,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    fn refresh_invitation_token(
        &self,
        id: &InvitationId,
        new_nonce: String,
        new_expiry_ms: i64,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    /// Atomically: consume nonce, mark invitation accepted, add membership.
    fn accept_invitation_existing_user(
        &self,
        user_id: &UserId,
        acceptance: InvitationAcceptance,
        nonce: &str,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    // ----- Nonces -----

    /// Returns `Ok(())` on first use, `Err(AlreadyExists)` on replay.
    fn consume_nonce(
        &self,
        nonce: &str,
        purpose: &str,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<()>> + Send;

    // ----- SSO state (cross-request CSRF token) -----

    fn store_sso_state(&self, state: SsoState) -> impl Future<Output = StoreResult<()>> + Send;

    fn consume_sso_state(
        &self,
        state: &str,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<Option<SsoState>>> + Send;
}
