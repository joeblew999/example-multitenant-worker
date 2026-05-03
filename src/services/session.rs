use connectrpc::ConnectError;

use crate::auth::{MintSessionInput, mint_session};
use crate::billing::BillingProvider;
use crate::domain::{AuthMethod, BillingAccount, Organization, Role, User, UserId};
use crate::proto::workers::auth::v1::WhoamiInfo;
use crate::state::SharedState;
use crate::store::{BillingAccountWithRole, OrgWithRole, Repo};

use super::common::map_token_error;
use super::convert::build_whoami;

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
