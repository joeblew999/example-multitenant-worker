//! Storage-shape entities. Decoupled from generated proto types so the
//! store layer doesn't carry buffa's internals (unknown-fields, cached
//! sizes) and so SSO/billing internals never leak onto the wire.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::enums::{Role, ScopeKind, ScopeTarget};
use super::ids::{BillingAccountId, IdentityId, InvitationId, InvoiceId, OrgId, UserId};

pub fn personal_display_name(email: &str) -> String {
    format!("{email} (personal)")
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub email: String,
    pub email_verified: bool,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    pub id: IdentityId,
    pub user_id: UserId,
    /// String form of `IdentityProvider` (`password`, `github`, `google`,
    /// `sso:<idp_id>`).
    pub provider: String,
    pub provider_user_id: String,
    /// Argon2id PHC string for password identities; `None` otherwise.
    pub secret: Option<String>,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BillingAccount {
    pub id: BillingAccountId,
    pub display_name: String,
    pub personal: bool,
    pub owner_user_id: Option<UserId>,
    pub auto_join_domain: Option<String>,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Organization {
    pub id: OrgId,
    pub display_name: String,
    pub billing_account_id: BillingAccountId,
    pub personal: bool,
    pub owner_user_id: Option<UserId>,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BillingMembership {
    pub user_id: UserId,
    pub billing_account_id: BillingAccountId,
    pub role: Role,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrgMembership {
    pub user_id: UserId,
    pub org_id: OrgId,
    pub role: Role,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SsoConfig {
    pub scope_kind: ScopeKind,
    pub scope_id: String,
    pub idp_id: String,
    pub kind: String,
    pub login_url: String,
    pub break_glass_user_id: UserId,
    pub required: bool,
    pub updated_at_ms: i64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Declined,
    Revoked,
    Expired,
}

impl InvitationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            InvitationStatus::Pending => "pending",
            InvitationStatus::Accepted => "accepted",
            InvitationStatus::Declined => "declined",
            InvitationStatus::Revoked => "revoked",
            InvitationStatus::Expired => "expired",
        }
    }
}

impl FromStr for InvitationStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(InvitationStatus::Pending),
            "accepted" => Ok(InvitationStatus::Accepted),
            "declined" => Ok(InvitationStatus::Declined),
            "revoked" => Ok(InvitationStatus::Revoked),
            "expired" => Ok(InvitationStatus::Expired),
            other => Err(format!("unknown invitation status: {other}")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invitation {
    pub id: InvitationId,
    pub scope: ScopeTarget,
    pub email: String,
    pub role: Role,
    pub inviter_user_id: Option<UserId>,
    /// IdP frozen at issuance time when the scope was SSO-protected.
    pub required_idp: Option<String>,
    /// Current macaroon nonce; updated on resend so older tokens stop
    /// redeeming.
    pub nonce: String,
    pub expires_at_ms: i64,
    pub status: InvitationStatus,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    pub billing_account_id: BillingAccountId,
    pub plan: String,
    pub status: String,
    pub payment_method_token: Option<String>,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invoice {
    pub id: InvoiceId,
    pub billing_account_id: BillingAccountId,
    pub amount_cents: i64,
    pub currency: String,
    pub status: String,
    pub issued_at_ms: i64,
}

/// Pending SSO state stamped during `SsoStart`, consumed by `SsoComplete`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SsoState {
    pub state: String,
    pub scope_hint: String,
    pub expected_idp: Option<String>,
    pub created_at_ms: i64,
    pub expires_at_ms: i64,
}
