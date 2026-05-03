use connectrpc::ConnectError;

use crate::auth::{PasswordError, TokenError};
use crate::billing::BillingProvider;
use crate::domain::{Identity, IdentityProvider, Role, ScopeKind, User, UserId};
use crate::state::SharedState;
use crate::store::{Repo, StoreError};

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

pub async fn fetch_user<R: Repo, B: BillingProvider>(
    state: &SharedState<R, B>,
    user_id: &UserId,
) -> Result<User, ConnectError> {
    state
        .repo
        .get_user(user_id)
        .await?
        .ok_or_else(|| ConnectError::unauthenticated("session user no longer exists"))
}

pub async fn fetch_password_identity<R: Repo, B: BillingProvider>(
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
