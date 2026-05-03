use connectrpc::ConnectError;

use crate::auth::SessionContext;
use crate::billing::BillingProvider;
use crate::domain::{BillingAccountId, OrgId, Organization, Role, ScopeKind, SsoConfig};
use crate::state::SharedState;
use crate::store::Repo;

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
