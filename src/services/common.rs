use connectrpc::ConnectError;
use uuid::Uuid;

use crate::auth::{PasswordError, TokenError};
use crate::billing::BillingProvider;
use crate::domain::{
    BillingAccountId, Identity, IdentityProvider, Invitation, InvitationId, InvitationStatus, Role,
    ScopeKind, ScopeTarget, User, UserId,
};
use crate::services::auth::build_invitation_token;
use crate::services::authz::resolve_required_sso;
use crate::state::SharedState;
use crate::store::{Repo, StoreError, UserRepo};

pub fn map_token_error(e: TokenError) -> ConnectError {
    match e {
        TokenError::InvalidSignature => ConnectError::unauthenticated("invalid token signature"),
        TokenError::Malformed(s) => ConnectError::unauthenticated(format!("malformed token: {s}")),
        TokenError::Expired => ConnectError::unauthenticated("token expired"),
        TokenError::WrongPurpose { .. } => ConnectError::permission_denied(e.to_string()),
        TokenError::MissingCaveat(_) | TokenError::InvalidCaveat(_) => {
            ConnectError::unauthenticated(e.to_string())
        }
    }
}

pub fn map_already_exists_as_precondition(e: StoreError, msg: &'static str) -> ConnectError {
    match e {
        StoreError::AlreadyExists(_) => ConnectError::failed_precondition(msg),
        other => other.into(),
    }
}

pub fn map_password_error(e: PasswordError) -> ConnectError {
    match e {
        PasswordError::TooShort(_) => ConnectError::invalid_argument(e.to_string()),
        PasswordError::Pwned => ConnectError::invalid_argument(
            "password appears in known-breached corpus; pick another",
        ),
        PasswordError::Mismatch => ConnectError::unauthenticated("password mismatch"),
        PasswordError::HashFailed(_) | PasswordError::InvalidHash(_) => {
            ConnectError::internal(e.to_string())
        }
        PasswordError::PolicyError(_) => ConnectError::invalid_argument(e.to_string()),
    }
}

pub fn validate_pending_invitation(
    stored: &crate::domain::Invitation,
    expected_nonce: &str,
) -> Result<(), ConnectError> {
    if stored.nonce != expected_nonce {
        return Err(ConnectError::failed_precondition(
            "invitation has been re-issued; use the latest link",
        ));
    }
    if stored.status != crate::domain::InvitationStatus::Pending {
        return Err(ConnectError::failed_precondition(format!(
            "invitation status is {}",
            stored.status.as_str()
        )));
    }
    Ok(())
}

pub fn parse_scope_kind(s: &str) -> Result<ScopeKind, ConnectError> {
    s.parse::<ScopeKind>()
        .map_err(ConnectError::invalid_argument)
}

pub fn role_from_i32(v: i32) -> Result<Role, ConnectError> {
    match v {
        1 => Ok(Role::Owner),
        2 => Ok(Role::Member),
        _ => Err(ConnectError::invalid_argument(format!(
            "unknown role value: {v}"
        ))),
    }
}

pub fn email_domain(email: &str) -> Option<String> {
    let (_, dom) = email.split_once('@')?;
    let dom = dom.trim();
    if dom.is_empty() || dom.contains('@') {
        return None;
    }
    Some(dom.to_ascii_lowercase())
}

pub fn validate_email(email: &str) -> Result<(), ConnectError> {
    let trimmed = email.trim();
    if trimmed.is_empty() {
        return Err(ConnectError::invalid_argument("email is required"));
    }
    let (local, domain) = trimmed
        .split_once('@')
        .ok_or_else(|| ConnectError::invalid_argument("email must contain '@'"))?;
    if local.is_empty() || domain.is_empty() {
        return Err(ConnectError::invalid_argument("malformed email"));
    }
    if domain.contains('@') {
        return Err(ConnectError::invalid_argument("malformed email"));
    }
    Ok(())
}

pub async fn fetch_user<R: UserRepo, B: BillingProvider>(
    state: &SharedState<R, B>,
    user_id: &UserId,
) -> Result<User, ConnectError> {
    state
        .repo
        .get_user(user_id)
        .await?
        .ok_or_else(|| ConnectError::unauthenticated("session user no longer exists"))
}

pub async fn fetch_password_identity<R: UserRepo, B: BillingProvider>(
    state: &SharedState<R, B>,
    user_id: &UserId,
    not_found: impl FnOnce() -> ConnectError,
) -> Result<Identity, ConnectError> {
    state
        .repo
        .list_identities(user_id)
        .await?
        .into_iter()
        .find(|i| i.provider == IdentityProvider::PASSWORD)
        .ok_or_else(not_found)
}

pub struct InviteMemberResult {
    pub invitation_id: InvitationId,
    pub invite_token: String,
}

pub struct InviteMemberParams<'a> {
    pub email: &'a str,
    pub role_i32: i32,
    pub inviter_user_id: &'a UserId,
    pub scope: ScopeTarget,
    pub billing_account_id: &'a BillingAccountId,
}

pub async fn execute_invite_member<R: Repo, B: BillingProvider>(
    state: &SharedState<R, B>,
    params: InviteMemberParams<'_>,
) -> Result<InviteMemberResult, ConnectError> {
    validate_email(params.email)?;
    let role = role_from_i32(params.role_i32)?;
    let now_ms = state.clock.now_ms();
    let now_unix = state.clock.now_unix_seconds();

    let org_id_for_sso = match &params.scope {
        ScopeTarget::Org(id) => Some(id),
        _ => None,
    };
    let required_idp = resolve_required_sso(state, params.billing_account_id, org_id_for_sso)
        .await?
        .filter(|c| c.required)
        .map(|c| c.idp_id);

    let invitation_id = InvitationId::new();
    let nonce = format!("inv_{}", Uuid::new_v4());
    let expires_at_ms = now_ms + state.config.invitation_ttl_seconds * 1000;

    let existing = state
        .repo
        .list_pending_invitation_by_scope_email(
            params.scope.scope_kind(),
            params.scope.scope_id_str(),
            params.email,
        )
        .await?;

    let invitation = match existing {
        Some(mut inv) => {
            state
                .repo
                .refresh_invitation_token(&inv.id, nonce.clone(), expires_at_ms)
                .await?;
            inv.nonce = nonce.clone();
            inv.expires_at_ms = expires_at_ms;
            inv.role = role;
            inv.required_idp = required_idp.clone();
            inv.status = InvitationStatus::Pending;
            inv
        }
        None => {
            let inv = Invitation {
                id: invitation_id.clone(),
                scope: params.scope,
                email: params.email.to_owned(),
                role,
                inviter_user_id: Some(params.inviter_user_id.clone()),
                required_idp: required_idp.clone(),
                nonce: nonce.clone(),
                expires_at_ms,
                status: InvitationStatus::Pending,
                created_at_ms: now_ms,
            };
            state.repo.create_invitation(inv.clone()).await?;
            inv
        }
    };

    let token = build_invitation_token(
        state,
        &invitation.id,
        &invitation.email,
        invitation.scope.scope_kind(),
        invitation.scope.scope_id_str(),
        role,
        required_idp.as_deref(),
        &nonce,
        now_unix,
    )?;

    Ok(InviteMemberResult {
        invitation_id: invitation.id,
        invite_token: token,
    })
}
