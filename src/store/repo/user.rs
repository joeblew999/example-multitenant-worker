use std::future::Future;

use crate::domain::{BillingAccountId, Identity, IdentityId, User, UserId};
use crate::store::error::StoreResult;

use super::invitation::InvitationAcceptance;

#[derive(Clone, Debug)]
pub struct NewSsoUser {
    pub email: String,
    pub idp_id: String,
    pub idp_user_id: String,
}

#[derive(Clone, Debug)]
pub struct NewPasswordUser {
    pub email: String,
    pub password_hash: String,
}

pub trait UserRepo: Send + Sync + 'static {
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

    fn link_identity(
        &self,
        user_id: &UserId,
        provider: &str,
        provider_user_id: &str,
        secret: Option<String>,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<Identity>> + Send;
}
