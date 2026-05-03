//! `workers.invitation.v1.InvitationService` implementation.

use connectrpc::{ConnectError, RequestContext, Response, ServiceResult};
use uuid::Uuid;

use crate::auth::verify_invitation;
use crate::billing::BillingProvider;
use crate::domain::{
    Invitation, InvitationId, InvitationStatus, ScopeKind, ScopeTarget, TokenPurpose,
};
use crate::middleware::require_session;
use crate::proto::workers::invitation::v1::{
    AcceptInvitationResponse, DeclineInvitationResponse, GetInvitationResponse,
    Invitation as InvitationPb, InvitationService, InvitationStatus as InvitationStatusPb,
    ListPendingInvitationsResponse, OwnedAcceptInvitationRequestView,
    OwnedDeclineInvitationRequestView, OwnedGetInvitationRequestView,
    OwnedListPendingInvitationsRequestView, OwnedResendInvitationRequestView,
    OwnedRevokeInvitationRequestView, ResendInvitationResponse, RevokeInvitationResponse,
    Role as RolePb, ScopeKind as ScopeKindPb,
};
use crate::services::auth::build_invitation_token;
use crate::services::authz::require_org_or_billing_owner;
use crate::services::common::{
    fetch_user, map_already_exists_as_precondition, map_token_error, validate_pending_invitation,
};
use crate::state::SharedState;
use crate::store::{InvitationAcceptance, Repo};

pub struct InvitationServer<R: Repo, B: BillingProvider> {
    state: SharedState<R, B>,
}

impl<R: Repo, B: BillingProvider> InvitationServer<R, B> {
    pub fn new(state: SharedState<R, B>) -> Self {
        Self { state }
    }

    async fn scope_display_name(&self, invitation: &Invitation) -> Result<String, ConnectError> {
        match &invitation.scope {
            ScopeTarget::Billing(id) => {
                let acct = self.state.repo.get_billing_account(id).await?;
                Ok(acct.map(|a| a.display_name).unwrap_or_default())
            }
            ScopeTarget::Org(id) => {
                let org = self.state.repo.get_organization(id).await?;
                Ok(org.map(|o| o.display_name).unwrap_or_default())
            }
        }
    }

    async fn invitation_to_pb(&self, inv: Invitation) -> Result<InvitationPb, ConnectError> {
        let display_name = self.scope_display_name(&inv).await?;
        Ok(invitation_to_pb_with_display(inv, display_name))
    }
}

fn scope_kind_to_pb(sk: ScopeKind) -> buffa::EnumValue<ScopeKindPb> {
    match sk {
        ScopeKind::Billing => ScopeKindPb::SCOPE_KIND_BILLING.into(),
        ScopeKind::Org => ScopeKindPb::SCOPE_KIND_ORG.into(),
    }
}

fn role_to_inv_pb(r: crate::domain::Role) -> buffa::EnumValue<RolePb> {
    match r {
        crate::domain::Role::Owner => RolePb::ROLE_OWNER.into(),
        crate::domain::Role::Member => RolePb::ROLE_MEMBER.into(),
    }
}

fn invitation_status_to_pb(s: InvitationStatus) -> buffa::EnumValue<InvitationStatusPb> {
    match s {
        InvitationStatus::Pending => InvitationStatusPb::INVITATION_STATUS_PENDING.into(),
        InvitationStatus::Accepted => InvitationStatusPb::INVITATION_STATUS_ACCEPTED.into(),
        InvitationStatus::Declined => InvitationStatusPb::INVITATION_STATUS_DECLINED.into(),
        InvitationStatus::Revoked => InvitationStatusPb::INVITATION_STATUS_REVOKED.into(),
        InvitationStatus::Expired => InvitationStatusPb::INVITATION_STATUS_EXPIRED.into(),
    }
}

fn invitation_to_pb_with_display(inv: Invitation, scope_display_name: String) -> InvitationPb {
    let scope_kind = scope_kind_to_pb(inv.scope.scope_kind());
    let scope_id = inv.scope.into_scope_id_string();
    InvitationPb {
        id: inv.id.to_string(),
        email: inv.email,
        scope_kind,
        scope_id,
        scope_display_name,
        role: role_to_inv_pb(inv.role),
        inviter_user_id: inv
            .inviter_user_id
            .map(|u| u.to_string())
            .unwrap_or_default(),
        expires_at: crate::services::convert::ms_to_timestamp(inv.expires_at_ms),
        status: invitation_status_to_pb(inv.status),
        required_idp: inv.required_idp,
        ..Default::default()
    }
}

impl<R: Repo, B: BillingProvider> InvitationService for InvitationServer<R, B> {
    async fn get_invitation(
        &self,
        _ctx: RequestContext,
        request: OwnedGetInvitationRequestView,
    ) -> ServiceResult<GetInvitationResponse> {
        let now_unix = self.state.clock.now_unix_seconds();
        let parsed = verify_invitation(&self.state.keyring, now_unix, request.token)
            .map_err(map_token_error)?;
        let stored = self
            .state
            .repo
            .get_invitation(&parsed.invitation_id)
            .await?
            .ok_or_else(|| ConnectError::not_found("invitation not found"))?;
        if stored.nonce != parsed.nonce {
            return Err(ConnectError::failed_precondition(
                "invitation has been re-issued; use the latest link",
            ));
        }
        let pb = self.invitation_to_pb(stored).await?;
        Response::ok(GetInvitationResponse {
            invitation: buffa::MessageField::some(pb),
            ..Default::default()
        })
    }

    async fn accept_invitation(
        &self,
        ctx: RequestContext,
        request: OwnedAcceptInvitationRequestView,
    ) -> ServiceResult<AcceptInvitationResponse> {
        let session = require_session(&ctx)?;
        let now_ms = self.state.clock.now_ms();
        let now_unix = self.state.clock.now_unix_seconds();
        let parsed = verify_invitation(&self.state.keyring, now_unix, request.token)
            .map_err(map_token_error)?;
        let user = fetch_user(&self.state, &session.user).await?;
        if !parsed.email.eq_ignore_ascii_case(&user.email) {
            return Err(ConnectError::permission_denied(
                "this invitation was issued to a different email",
            ));
        }
        let stored = self
            .state
            .repo
            .get_invitation(&parsed.invitation_id)
            .await?
            .ok_or_else(|| ConnectError::not_found("invitation no longer exists"))?;
        validate_pending_invitation(&stored, &parsed.nonce)?;
        // SSO precedence requirement: if the invite carried a required_idp,
        // the current session's auth_method must match.
        if let Some(idp) = parsed.required_idp.as_deref()
            && !matches!(&session.auth_method, crate::domain::AuthMethod::Sso(s) if s == idp)
        {
            return Err(ConnectError::permission_denied(format!(
                "this invitation requires SSO via {idp}"
            )));
        }
        let acceptance = InvitationAcceptance {
            invitation_id: parsed.invitation_id.clone(),
            target: ScopeTarget::from_parts(parsed.scope_kind, parsed.scope_id.clone()),
            role: parsed.role,
        };
        self.state
            .repo
            .accept_invitation_existing_user(&user.id, acceptance.clone(), &parsed.nonce, now_ms)
            .await
            .map_err(|e| map_already_exists_as_precondition(e, "invitation already accepted"))?;
        let mut invitation = stored;
        invitation.status = InvitationStatus::Accepted;
        let pb = self.invitation_to_pb(invitation).await?;
        Response::ok(AcceptInvitationResponse {
            invitation: buffa::MessageField::some(pb),
            scope_kind: scope_kind_to_pb(parsed.scope_kind),
            scope_id: parsed.scope_id,
            role: role_to_inv_pb(parsed.role),
            ..Default::default()
        })
    }

    async fn decline_invitation(
        &self,
        _ctx: RequestContext,
        request: OwnedDeclineInvitationRequestView,
    ) -> ServiceResult<DeclineInvitationResponse> {
        let now_ms = self.state.clock.now_ms();
        let now_unix = self.state.clock.now_unix_seconds();
        let parsed = verify_invitation(&self.state.keyring, now_unix, request.token)
            .map_err(map_token_error)?;
        // Decline consumes the nonce so a re-presented link no longer
        // resolves.
        self.state
            .repo
            .consume_nonce(&parsed.nonce, TokenPurpose::Invitation.as_str(), now_ms)
            .await
            .map_err(|e| {
                map_already_exists_as_precondition(e, "invitation already responded to")
            })?;
        self.state
            .repo
            .update_invitation_status(&parsed.invitation_id, InvitationStatus::Declined)
            .await?;
        Response::ok(DeclineInvitationResponse::default())
    }

    async fn list_pending_invitations(
        &self,
        ctx: RequestContext,
        _request: OwnedListPendingInvitationsRequestView,
    ) -> ServiceResult<ListPendingInvitationsResponse> {
        let session = require_session(&ctx)?;
        let user = fetch_user(&self.state, &session.user).await?;
        let invs = self
            .state
            .repo
            .list_pending_invitations_by_email(&user.email)
            .await?;
        let billing_ids: Vec<&str> = invs
            .iter()
            .filter(|i| i.scope.scope_kind() == ScopeKind::Billing)
            .map(|i| i.scope.scope_id_str())
            .collect();
        let org_ids: Vec<&str> = invs
            .iter()
            .filter(|i| i.scope.scope_kind() == ScopeKind::Org)
            .map(|i| i.scope.scope_id_str())
            .collect();
        let (billings, orgs) = futures::try_join!(
            self.state.repo.get_billing_accounts_by_ids(&billing_ids),
            self.state.repo.get_organizations_by_ids(&org_ids),
        )?;
        let out: Vec<InvitationPb> = invs
            .into_iter()
            .map(|inv| {
                let display_name = match &inv.scope {
                    ScopeTarget::Billing(id) => billings
                        .get(id.as_str())
                        .map(|b| b.display_name.clone())
                        .unwrap_or_default(),
                    ScopeTarget::Org(id) => orgs
                        .get(id.as_str())
                        .map(|o| o.display_name.clone())
                        .unwrap_or_default(),
                };
                invitation_to_pb_with_display(inv, display_name)
            })
            .collect();
        Response::ok(ListPendingInvitationsResponse {
            invitations: out,
            ..Default::default()
        })
    }

    async fn resend_invitation(
        &self,
        ctx: RequestContext,
        request: OwnedResendInvitationRequestView,
    ) -> ServiceResult<ResendInvitationResponse> {
        let session = require_session(&ctx)?;
        let invitation_id = InvitationId::from(request.invitation_id);
        let mut invitation = self
            .state
            .repo
            .get_invitation(&invitation_id)
            .await?
            .ok_or_else(|| ConnectError::not_found("invitation not found"))?;
        self.require_inviter_or_owner(&session, &invitation).await?;

        let now_ms = self.state.clock.now_ms();
        let now_unix = self.state.clock.now_unix_seconds();
        let new_nonce = format!("inv_{}", Uuid::new_v4());
        let new_expiry = now_ms + self.state.config.invitation_ttl_seconds * 1000;
        self.state
            .repo
            .refresh_invitation_token(&invitation.id, new_nonce.clone(), new_expiry)
            .await?;
        invitation.nonce = new_nonce.clone();
        invitation.expires_at_ms = new_expiry;
        invitation.status = InvitationStatus::Pending;

        let token = build_invitation_token(
            &self.state,
            &invitation.id,
            &invitation.email,
            invitation.scope.scope_kind(),
            invitation.scope.scope_id_str(),
            invitation.role,
            invitation.required_idp.as_deref(),
            &new_nonce,
            now_unix,
        )?;
        let pb = self.invitation_to_pb(invitation).await?;
        Response::ok(ResendInvitationResponse {
            invitation: buffa::MessageField::some(pb),
            invite_token_for_demo: token,
            ..Default::default()
        })
    }

    async fn revoke_invitation(
        &self,
        ctx: RequestContext,
        request: OwnedRevokeInvitationRequestView,
    ) -> ServiceResult<RevokeInvitationResponse> {
        let session = require_session(&ctx)?;
        let invitation_id = InvitationId::from(request.invitation_id);
        let invitation = self
            .state
            .repo
            .get_invitation(&invitation_id)
            .await?
            .ok_or_else(|| ConnectError::not_found("invitation not found"))?;
        self.require_inviter_or_owner(&session, &invitation).await?;
        // Rotate the nonce to invalidate any outstanding link, then mark
        // the invitation as revoked.
        let new_nonce = format!("inv_revoked_{}", Uuid::new_v4());
        self.state
            .repo
            .refresh_invitation_token(&invitation.id, new_nonce, 0)
            .await?;
        self.state
            .repo
            .update_invitation_status(&invitation.id, InvitationStatus::Revoked)
            .await?;
        Response::ok(RevokeInvitationResponse::default())
    }
}

impl<R: Repo, B: BillingProvider> InvitationServer<R, B> {
    async fn require_inviter_or_owner(
        &self,
        session: &crate::auth::SessionContext,
        invitation: &Invitation,
    ) -> Result<(), ConnectError> {
        if invitation.inviter_user_id.as_ref() == Some(&session.user) {
            return Ok(());
        }
        match &invitation.scope {
            ScopeTarget::Billing(billing_id) => {
                if let Some(mem) = self
                    .state
                    .repo
                    .get_billing_membership(&session.user, billing_id)
                    .await?
                    && mem.role == crate::domain::Role::Owner
                {
                    return Ok(());
                }
                Err(ConnectError::permission_denied(
                    "only the inviter or a scope owner can manage this invitation",
                ))
            }
            ScopeTarget::Org(org_id) => {
                let org = self
                    .state
                    .repo
                    .get_organization(org_id)
                    .await?
                    .ok_or_else(|| ConnectError::not_found("organization not found"))?;
                require_org_or_billing_owner(&self.state, session, &org).await
            }
        }
    }
}
