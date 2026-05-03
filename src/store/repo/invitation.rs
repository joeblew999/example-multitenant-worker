use std::future::Future;

use crate::domain::{
    Invitation, InvitationId, InvitationStatus, Role, ScopeKind, ScopeTarget, UserId,
};
use crate::store::error::StoreResult;

#[derive(Clone, Debug)]
pub struct InvitationAcceptance {
    pub invitation_id: InvitationId,
    pub target: ScopeTarget,
    pub role: Role,
}

pub trait InvitationRepo: Send + Sync + 'static {
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

    fn accept_invitation_existing_user(
        &self,
        user_id: &UserId,
        acceptance: InvitationAcceptance,
        nonce: &str,
        now_ms: i64,
    ) -> impl Future<Output = StoreResult<()>> + Send;
}
