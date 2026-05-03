//! `workers.org.v1.OrgService` implementation. Organizations are
//! immutable to their parent billing account (no transfers).

use connectrpc::{ConnectError, RequestContext, Response, ServiceResult};

use crate::auth::SessionContext;
use crate::billing::BillingProvider;
use crate::domain::{
    BillingAccountId, OrgId, Organization, Role, ScopeKind, ScopeTarget, SsoConfig, UserId,
};
use crate::middleware::require_session;
use crate::proto::workers::org::v1::{
    ConfigureSsoResponse, CreateOrganizationResponse, DeleteOrganizationResponse,
    GetOrganizationResponse, InviteMemberResponse, ListOrganizationsResponse, OrgService,
    Organization as OrganizationPb, OwnedConfigureSsoRequestView,
    OwnedCreateOrganizationRequestView, OwnedDeleteOrganizationRequestView,
    OwnedGetOrganizationRequestView, OwnedInviteMemberRequestView,
    OwnedListOrganizationsRequestView, OwnedRemoveMemberRequestView,
    OwnedUpdateMemberRoleRequestView, RemoveMemberResponse, UpdateMemberRoleResponse,
};
use crate::services::authz::{require_billing_owner, require_org_or_billing_owner};
use crate::services::common::{InviteMemberParams, execute_invite_member, role_from_i32};
use crate::services::convert::{sso_kind_to_domain, sso_to_pb};
use crate::state::SharedState;
use crate::store::Repo;

pub struct OrgServer<R: Repo, B: BillingProvider> {
    state: SharedState<R, B>,
}

impl<R: Repo, B: BillingProvider> OrgServer<R, B> {
    pub fn new(state: SharedState<R, B>) -> Self {
        Self { state }
    }

    async fn require_org_visibility(
        &self,
        session: &SessionContext,
        org: &Organization,
    ) -> Result<(), ConnectError> {
        if self
            .state
            .repo
            .get_org_membership(&session.user, &org.id)
            .await?
            .is_some()
        {
            return Ok(());
        }
        if self
            .state
            .repo
            .get_billing_membership(&session.user, &org.billing_account_id)
            .await?
            .is_some()
        {
            return Ok(());
        }
        Err(ConnectError::permission_denied(
            "no access to this organization",
        ))
    }

    async fn org_to_pb(&self, o: Organization) -> Result<OrganizationPb, ConnectError> {
        let sso = self
            .state
            .repo
            .get_sso_config(ScopeKind::Org, o.id.as_str())
            .await?;
        Ok(org_to_pb_with_sso(o, sso))
    }
}

fn org_to_pb_with_sso(o: Organization, sso: Option<SsoConfig>) -> OrganizationPb {
    OrganizationPb {
        id: o.id.to_string(),
        display_name: o.display_name,
        billing_account_id: o.billing_account_id.to_string(),
        personal: o.personal,
        owner_user_id: o.owner_user_id.map(|u| u.to_string()).unwrap_or_default(),
        sso: sso
            .map(sso_to_pb)
            .map(buffa::MessageField::some)
            .unwrap_or_default(),
        ..Default::default()
    }
}

impl<R: Repo, B: BillingProvider> OrgService for OrgServer<R, B> {
    async fn create_organization(
        &self,
        ctx: RequestContext,
        request: OwnedCreateOrganizationRequestView,
    ) -> ServiceResult<CreateOrganizationResponse> {
        let session = require_session(&ctx)?;
        let billing_id = BillingAccountId::from(request.billing_account_id);
        require_billing_owner(&self.state, &session, &billing_id).await?;
        let now_ms = self.state.clock.now_ms();
        let display_name = request.display_name.trim();
        if display_name.is_empty() {
            return Err(ConnectError::invalid_argument("display_name is required"));
        }
        let org = self
            .state
            .repo
            .create_organization(display_name.to_owned(), billing_id.clone(), now_ms)
            .await?;
        self.state
            .repo
            .add_org_membership(&session.user, &org.id, Role::Owner, now_ms)
            .await?;
        let pb = self.org_to_pb(org).await?;
        Response::ok(CreateOrganizationResponse {
            organization: buffa::MessageField::some(pb),
            ..Default::default()
        })
    }

    async fn get_organization(
        &self,
        ctx: RequestContext,
        request: OwnedGetOrganizationRequestView,
    ) -> ServiceResult<GetOrganizationResponse> {
        let session = require_session(&ctx)?;
        let id = OrgId::from(request.id);
        let org = self
            .state
            .repo
            .get_organization(&id)
            .await?
            .ok_or_else(|| ConnectError::not_found("organization not found"))?;
        self.require_org_visibility(&session, &org).await?;
        let pb = self.org_to_pb(org).await?;
        Response::ok(GetOrganizationResponse {
            organization: buffa::MessageField::some(pb),
            ..Default::default()
        })
    }

    async fn list_organizations(
        &self,
        ctx: RequestContext,
        request: OwnedListOrganizationsRequestView,
    ) -> ServiceResult<ListOrganizationsResponse> {
        let session = require_session(&ctx)?;
        let orgs = match request.billing_account_id {
            Some(s) if !s.is_empty() => {
                let billing_id = BillingAccountId::from(s);
                // Visibility: any membership in the billing account exposes
                // the org list.
                let mem = self
                    .state
                    .repo
                    .get_billing_membership(&session.user, &billing_id)
                    .await?;
                if mem.is_none() {
                    return Err(ConnectError::permission_denied(
                        "not a member of this billing account",
                    ));
                }
                self.state
                    .repo
                    .list_organizations_for_billing(&billing_id)
                    .await?
            }
            _ => self
                .state
                .repo
                .list_organizations_for_user(&session.user)
                .await?
                .into_iter()
                .map(|(o, _)| o)
                .collect(),
        };
        let scope_ids: Vec<&str> = orgs.iter().map(|o| o.id.as_str()).collect();
        let mut sso_by_id = self
            .state
            .repo
            .list_sso_configs_by_scope_ids(ScopeKind::Org, &scope_ids)
            .await?;
        let out: Vec<OrganizationPb> = orgs
            .into_iter()
            .map(|o| {
                let sso = sso_by_id.remove(o.id.as_str());
                org_to_pb_with_sso(o, sso)
            })
            .collect();
        Response::ok(ListOrganizationsResponse {
            organizations: out,
            ..Default::default()
        })
    }

    async fn invite_member(
        &self,
        ctx: RequestContext,
        request: OwnedInviteMemberRequestView,
    ) -> ServiceResult<InviteMemberResponse> {
        let session = require_session(&ctx)?;
        let org_id = OrgId::from(request.org_id);
        let org = self
            .state
            .repo
            .get_organization(&org_id)
            .await?
            .ok_or_else(|| ConnectError::not_found("organization not found"))?;
        require_org_or_billing_owner(&self.state, &session, &org).await?;

        if let Some(target_user) = self.state.repo.get_user_by_email(request.email).await?
            && self
                .state
                .repo
                .get_org_membership(&target_user.id, &org_id)
                .await?
                .is_some()
        {
            return Err(ConnectError::failed_precondition(
                "user is already a member of this org",
            ));
        }

        let result = execute_invite_member(
            &self.state,
            InviteMemberParams {
                email: request.email,
                role_i32: request.role.to_i32(),
                inviter_user_id: &session.user,
                scope: ScopeTarget::Org(org_id.clone()),
                billing_account_id: &org.billing_account_id,
            },
        )
        .await?;

        Response::ok(InviteMemberResponse {
            invitation_id: result.invitation_id.to_string(),
            invite_token_for_demo: result.invite_token,
            ..Default::default()
        })
    }

    async fn remove_member(
        &self,
        ctx: RequestContext,
        request: OwnedRemoveMemberRequestView,
    ) -> ServiceResult<RemoveMemberResponse> {
        let session = require_session(&ctx)?;
        let org_id = OrgId::from(request.org_id);
        let org = self
            .state
            .repo
            .get_organization(&org_id)
            .await?
            .ok_or_else(|| ConnectError::not_found("organization not found"))?;
        require_org_or_billing_owner(&self.state, &session, &org).await?;
        let target_user = UserId::from(request.user_id);
        self.state
            .repo
            .remove_org_membership(&target_user, &org_id)
            .await?;
        Response::ok(RemoveMemberResponse::default())
    }

    async fn update_member_role(
        &self,
        ctx: RequestContext,
        request: OwnedUpdateMemberRoleRequestView,
    ) -> ServiceResult<UpdateMemberRoleResponse> {
        let session = require_session(&ctx)?;
        let org_id = OrgId::from(request.org_id);
        let org = self
            .state
            .repo
            .get_organization(&org_id)
            .await?
            .ok_or_else(|| ConnectError::not_found("organization not found"))?;
        require_org_or_billing_owner(&self.state, &session, &org).await?;
        let target_user = UserId::from(request.user_id);
        let role = role_from_i32(request.role.to_i32())?;
        self.state
            .repo
            .update_org_membership_role(&target_user, &org_id, role)
            .await?;
        Response::ok(UpdateMemberRoleResponse::default())
    }

    async fn configure_sso(
        &self,
        ctx: RequestContext,
        request: OwnedConfigureSsoRequestView,
    ) -> ServiceResult<ConfigureSsoResponse> {
        let session = require_session(&ctx)?;
        let org_id = OrgId::from(request.org_id);
        let org = self
            .state
            .repo
            .get_organization(&org_id)
            .await?
            .ok_or_else(|| ConnectError::not_found("organization not found"))?;
        require_org_or_billing_owner(&self.state, &session, &org).await?;
        let now_ms = self.state.clock.now_ms();
        let cfg = SsoConfig {
            scope_kind: ScopeKind::Org,
            scope_id: org_id.to_string(),
            idp_id: request.idp_id.to_owned(),
            kind: sso_kind_to_domain(request.kind.to_i32()),
            login_url: request.login_url.to_owned(),
            break_glass_user_id: session.user.clone(),
            required: request.required,
            updated_at_ms: now_ms,
        };
        self.state.repo.upsert_sso_config(cfg.clone()).await?;
        Response::ok(ConfigureSsoResponse {
            sso: buffa::MessageField::some(sso_to_pb(cfg)),
            ..Default::default()
        })
    }

    async fn delete_organization(
        &self,
        ctx: RequestContext,
        request: OwnedDeleteOrganizationRequestView,
    ) -> ServiceResult<DeleteOrganizationResponse> {
        let session = require_session(&ctx)?;
        let org_id = OrgId::from(request.org_id);
        let org = self
            .state
            .repo
            .get_organization(&org_id)
            .await?
            .ok_or_else(|| ConnectError::not_found("organization not found"))?;
        if org.personal {
            return Err(ConnectError::failed_precondition(
                "personal organizations are deleted with the user",
            ));
        }
        require_org_or_billing_owner(&self.state, &session, &org).await?;
        self.state.repo.delete_organization(&org_id).await?;
        Response::ok(DeleteOrganizationResponse::default())
    }
}
