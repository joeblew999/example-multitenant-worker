//! Helpers shared by service implementations: building `WhoamiInfo`,
//! mapping `TokenError`/`PasswordError` to ConnectErrors, resolving SSO
//! precedence (org-level wins over billing-level fallback), and shared
//! permission checks.

use connectrpc::ConnectError;

use crate::auth::{MintSessionInput, PasswordError, SessionContext, TokenError, mint_session};
use crate::billing::BillingProvider;
use crate::domain::{
    AuthMethod, BillingAccount, BillingAccountId, Identity, IdentityProvider, OrgId, Organization,
    Role, ScopeKind, SsoConfig, Subscription, User, UserId,
};
use crate::proto::workers::auth::v1::{
    AuthMethod as AuthMethodPb, AuthMethodKind as AuthMethodKindPb, MembershipSummary,
    Role as RolePb, WhoamiInfo,
};
use crate::proto::workers::billing::v1::{
    SsoConfig as SsoConfigPb, SsoKind as SsoKindPb, Subscription as SubscriptionPb,
    SubscriptionStatus as SubscriptionStatusPb,
};
use crate::state::SharedState;
use crate::store::{BillingAccountWithRole, OrgWithRole, Repo, StoreError};

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

/// Map `StoreError::AlreadyExists` to a typed `failed_precondition` (used
/// when a unique-constraint hit reflects a replayed nonce or duplicate
/// invitation rather than a server bug). Other store errors propagate via
/// the standard `From` impl.
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

pub fn role_to_pb(r: Role) -> buffa::EnumValue<RolePb> {
    match r {
        Role::Owner => RolePb::ROLE_OWNER.into(),
        Role::Member => RolePb::ROLE_MEMBER.into(),
    }
}

pub fn auth_method_to_pb(m: &AuthMethod) -> buffa::MessageField<AuthMethodPb> {
    let pb = match m {
        AuthMethod::Password => AuthMethodPb {
            kind: AuthMethodKindPb::AUTH_METHOD_KIND_PASSWORD.into(),
            ..Default::default()
        },
        AuthMethod::Sso(idp_id) => AuthMethodPb {
            kind: AuthMethodKindPb::AUTH_METHOD_KIND_SSO.into(),
            idp_id: idp_id.clone(),
            ..Default::default()
        },
    };
    buffa::MessageField::some(pb)
}

pub fn sso_kind_to_domain(v: i32) -> String {
    match v {
        1 => "oidc".to_owned(),
        2 => "saml".to_owned(),
        _ => format!("unknown:{v}"),
    }
}

pub fn sso_kind_from_domain(s: &str) -> buffa::EnumValue<SsoKindPb> {
    match s {
        "oidc" => SsoKindPb::SSO_KIND_OIDC.into(),
        "saml" => SsoKindPb::SSO_KIND_SAML.into(),
        _ => buffa::EnumValue::from(0),
    }
}

pub fn ms_to_timestamp(ms: i64) -> buffa::MessageField<buffa_types::google::protobuf::Timestamp> {
    let ts = buffa_types::google::protobuf::Timestamp::from_unix(
        ms / 1000,
        ((ms % 1000) * 1_000_000) as i32,
    );
    buffa::MessageField::some(ts)
}

pub fn subscription_status_to_pb(s: &str) -> buffa::EnumValue<SubscriptionStatusPb> {
    match s {
        "active" => SubscriptionStatusPb::SUBSCRIPTION_STATUS_ACTIVE.into(),
        "past_due" => SubscriptionStatusPb::SUBSCRIPTION_STATUS_PAST_DUE.into(),
        "canceled" => SubscriptionStatusPb::SUBSCRIPTION_STATUS_CANCELED.into(),
        "none" => SubscriptionStatusPb::SUBSCRIPTION_STATUS_NONE.into(),
        _ => buffa::EnumValue::from(0),
    }
}

pub fn invoice_status_to_pb(
    s: &str,
) -> buffa::EnumValue<crate::proto::workers::billing::v1::InvoiceStatus> {
    use crate::proto::workers::billing::v1::InvoiceStatus as InvoiceStatusPb;
    match s {
        "paid" => InvoiceStatusPb::INVOICE_STATUS_PAID.into(),
        "open" => InvoiceStatusPb::INVOICE_STATUS_OPEN.into(),
        "void" => InvoiceStatusPb::INVOICE_STATUS_VOID.into(),
        _ => buffa::EnumValue::from(0),
    }
}

pub fn membership_summary(scope_id: String, display_name: String, role: Role) -> MembershipSummary {
    MembershipSummary {
        scope_id,
        display_name,
        role: role_to_pb(role),
        ..Default::default()
    }
}

pub fn build_whoami(
    user: &User,
    billing: &BillingAccount,
    org: Option<&Organization>,
    role: Role,
    auth_method: &AuthMethod,
    billings: Vec<BillingAccountWithRole>,
    orgs: Vec<OrgWithRole>,
) -> WhoamiInfo {
    WhoamiInfo {
        user_id: user.id.to_string(),
        email: user.email.clone(),
        email_verified: user.email_verified,
        billing_account_id: billing.id.to_string(),
        org_id: org.map(|o| o.id.to_string()).unwrap_or_default(),
        role: role_to_pb(role),
        auth_method: auth_method_to_pb(auth_method),
        billing_memberships: billings
            .into_iter()
            .map(|(b, r)| membership_summary(b.id.to_string(), b.display_name, r))
            .collect(),
        org_memberships: orgs
            .into_iter()
            .map(|(o, r)| membership_summary(o.id.to_string(), o.display_name, r))
            .collect(),
        ..Default::default()
    }
}

pub async fn fetch_membership_lists<R: Repo, B: BillingProvider>(
    state: &SharedState<R, B>,
    user_id: &UserId,
) -> Result<(Vec<BillingAccountWithRole>, Vec<OrgWithRole>), ConnectError> {
    let (billings, orgs) = futures::try_join!(
        state.repo.list_billing_accounts_for_user(user_id),
        state.repo.list_organizations_for_user(user_id),
    )?;
    Ok((billings, orgs))
}

pub async fn build_whoami_async<R: Repo, B: BillingProvider>(
    state: &SharedState<R, B>,
    user: &User,
    billing: &BillingAccount,
    org: Option<&Organization>,
    role: Role,
    auth_method: &AuthMethod,
) -> Result<WhoamiInfo, ConnectError> {
    let (billings, orgs) = fetch_membership_lists(state, &user.id).await?;
    Ok(build_whoami(
        user,
        billing,
        org,
        role,
        auth_method,
        billings,
        orgs,
    ))
}

pub fn find_personal_billing(
    billings: &[BillingAccountWithRole],
) -> Result<BillingAccount, ConnectError> {
    billings
        .iter()
        .find(|(b, _)| b.personal)
        .map(|(b, _)| b.clone())
        .ok_or_else(|| ConnectError::internal("personal billing missing"))
}

/// Mint a session token + `WhoamiInfo` rooted on the user's personal
/// billing account. Used by the four entrypoints (signup, login,
/// sso_complete, consume_password_reset) that finalize an auth flow.
pub async fn issue_personal_session<R: Repo, B: BillingProvider>(
    state: &SharedState<R, B>,
    user: &User,
    auth_method: AuthMethod,
) -> Result<(String, WhoamiInfo), ConnectError> {
    let (billings, orgs) = fetch_membership_lists(state, &user.id).await?;
    let personal = find_personal_billing(&billings)?;
    let now_unix = state.clock.now_unix_seconds();
    let token = mint_session(
        &state.keyring,
        MintSessionInput {
            user_id: &user.id,
            email: &user.email,
            billing: &personal.id,
            org: None,
            role: Role::Owner,
            auth_method: &auth_method,
            now_unix,
            ttl_seconds: state.config.session_ttl_seconds,
        },
    )
    .map_err(map_token_error)?;
    let whoami = build_whoami(
        user,
        &personal,
        None,
        Role::Owner,
        &auth_method,
        billings,
        orgs,
    );
    Ok((token, whoami))
}

/// SSO precedence rule: org-level config wins; falls back to billing-level.
/// Returns `Ok(None)` when no scope has SSO configured.
pub async fn resolve_required_sso<R: Repo, B: BillingProvider>(
    state: &SharedState<R, B>,
    billing: &BillingAccountId,
    org: Option<&OrgId>,
) -> Result<Option<SsoConfig>, ConnectError> {
    if let Some(org_id) = org
        && let Some(cfg) = state
            .repo
            .get_sso_config(ScopeKind::Org, org_id.as_str())
            .await?
    {
        return Ok(Some(cfg));
    }
    let cfg = state
        .repo
        .get_sso_config(ScopeKind::Billing, billing.as_str())
        .await?;
    Ok(cfg)
}

/// Domain part of an email, lower-cased. Returns `None` for non-`a@b`
/// strings.
pub fn email_domain(email: &str) -> Option<String> {
    let (_, dom) = email.split_once('@')?;
    let dom = dom.trim();
    if dom.is_empty() || dom.contains('@') {
        return None;
    }
    Some(dom.to_ascii_lowercase())
}

/// Validate email shape: exactly one `@`, non-empty local & domain. Not a
/// full RFC parse — same heuristic the demo SPA uses, the IdP / database
/// is the canonical source of truth.
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

/// Maps a missing user to `Unauthenticated` so a stale session whose user
/// has been deleted gets a clear response.
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

pub fn subscription_to_pb(s: Subscription) -> SubscriptionPb {
    SubscriptionPb {
        billing_account_id: s.billing_account_id.to_string(),
        plan: s.plan,
        status: subscription_status_to_pb(&s.status),
        payment_method_token: s.payment_method_token,
        updated_at: ms_to_timestamp(s.updated_at_ms),
        ..Default::default()
    }
}

pub fn sso_to_pb(s: SsoConfig) -> SsoConfigPb {
    SsoConfigPb {
        idp_id: s.idp_id,
        kind: sso_kind_from_domain(&s.kind),
        login_url: s.login_url,
        break_glass_user_id: s.break_glass_user_id.to_string(),
        required: s.required,
        ..Default::default()
    }
}

/// Fetch the user's PASSWORD identity. The error returned when the user
/// has no password identity differs by call site (unauthenticated for
/// login, failed_precondition for change/reset), so the caller supplies it.
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

pub async fn require_billing_member<R: Repo, B: BillingProvider>(
    state: &SharedState<R, B>,
    session: &SessionContext,
    billing_id: &BillingAccountId,
) -> Result<Role, ConnectError> {
    let mem = state
        .repo
        .get_billing_membership(&session.user, billing_id)
        .await?
        .ok_or_else(|| ConnectError::permission_denied("not a member of this billing account"))?;
    Ok(mem.role)
}

pub async fn require_billing_owner<R: Repo, B: BillingProvider>(
    state: &SharedState<R, B>,
    session: &SessionContext,
    billing_id: &BillingAccountId,
) -> Result<(), ConnectError> {
    if require_billing_member(state, session, billing_id).await? != Role::Owner {
        return Err(ConnectError::permission_denied(
            "billing-account owner role required",
        ));
    }
    Ok(())
}

/// Org owner OR billing-account owner of the org's parent billing.
pub async fn require_org_or_billing_owner<R: Repo, B: BillingProvider>(
    state: &SharedState<R, B>,
    session: &SessionContext,
    org: &Organization,
) -> Result<(), ConnectError> {
    if let Some(mem) = state
        .repo
        .get_org_membership(&session.user, &org.id)
        .await?
        && mem.role == Role::Owner
    {
        return Ok(());
    }
    if let Some(mem) = state
        .repo
        .get_billing_membership(&session.user, &org.billing_account_id)
        .await?
        && mem.role == Role::Owner
    {
        return Ok(());
    }
    Err(ConnectError::permission_denied(
        "org owner (or billing owner) role required",
    ))
}
