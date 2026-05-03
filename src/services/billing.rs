//! `workers.billing.v1.BillingService` implementation.
//!
//! Mutations gate on the caller being a billing-account *owner*. Read ops
//! gate on any membership. Personal billing accounts are non-deletable
//! and don't accept `SetAutoJoinDomain`.

use connectrpc::{ConnectError, RequestContext, Response, ServiceResult};
use uuid::Uuid;

use crate::billing::BillingProvider;
use crate::domain::{
    BillingAccount, BillingAccountId, Invitation, InvitationId, InvitationStatus, ScopeKind,
    SsoConfig, UserId,
};
use crate::middleware::require_session;
use crate::proto::workers::billing::v1::{
    BillingAccount as BillingAccountPb, BillingService, CancelSubscriptionResponse,
    ConfigureSsoResponse, DeleteBillingAccountResponse, GetBillingAccountResponse,
    GetSubscriptionResponse, InviteMemberResponse, Invoice as InvoicePb,
    ListBillingAccountsResponse, ListInvoicesResponse, OwnedCancelSubscriptionRequestView,
    OwnedConfigureSsoRequestView, OwnedDeleteBillingAccountRequestView,
    OwnedGetBillingAccountRequestView, OwnedGetSubscriptionRequestView,
    OwnedInviteMemberRequestView, OwnedListBillingAccountsRequestView,
    OwnedListInvoicesRequestView, OwnedRemoveMemberRequestView, OwnedSetAutoJoinDomainRequestView,
    OwnedSetPaymentMethodRequestView, OwnedUpdateMemberRoleRequestView, RemoveMemberResponse,
    SetAutoJoinDomainResponse, SetPaymentMethodResponse, UpdateMemberRoleResponse,
};
use crate::services::auth::build_invitation_token;
use crate::services::common::{
    invoice_status_to_pb, require_billing_member, require_billing_owner, resolve_required_sso,
    role_from_i32, sso_kind_to_domain, sso_to_pb, subscription_to_pb, validate_email,
};
use crate::state::SharedState;
use crate::store::Repo;

pub struct BillingServer<R: Repo, B: BillingProvider> {
    state: SharedState<R, B>,
}

impl<R: Repo, B: BillingProvider> BillingServer<R, B> {
    pub fn new(state: SharedState<R, B>) -> Self {
        Self { state }
    }

    async fn billing_to_pb(&self, b: BillingAccount) -> Result<BillingAccountPb, ConnectError> {
        let sso = self
            .state
            .repo
            .get_sso_config(ScopeKind::Billing, b.id.as_str())
            .await?;
        Ok(billing_to_pb_with_sso(b, sso))
    }
}

fn billing_to_pb_with_sso(b: BillingAccount, sso: Option<SsoConfig>) -> BillingAccountPb {
    BillingAccountPb {
        id: b.id.to_string(),
        display_name: b.display_name,
        personal: b.personal,
        owner_user_id: b.owner_user_id.map(|u| u.to_string()).unwrap_or_default(),
        auto_join_domain: b.auto_join_domain.unwrap_or_default(),
        sso: sso
            .map(sso_to_pb)
            .map(buffa::MessageField::some)
            .unwrap_or_default(),
        ..Default::default()
    }
}

impl<R: Repo, B: BillingProvider> BillingService for BillingServer<R, B> {
    async fn get_billing_account(
        &self,
        ctx: RequestContext,
        request: OwnedGetBillingAccountRequestView,
    ) -> ServiceResult<GetBillingAccountResponse> {
        let session = require_session(&ctx)?;
        let id = BillingAccountId::from(request.id);
        require_billing_member(&self.state, &session, &id).await?;
        let account = self
            .state
            .repo
            .get_billing_account(&id)
            .await?
            .ok_or_else(|| ConnectError::not_found("billing account not found"))?;
        let pb = self.billing_to_pb(account).await?;
        Response::ok(GetBillingAccountResponse {
            account: buffa::MessageField::some(pb),
            ..Default::default()
        })
    }

    async fn list_billing_accounts(
        &self,
        ctx: RequestContext,
        _request: OwnedListBillingAccountsRequestView,
    ) -> ServiceResult<ListBillingAccountsResponse> {
        let session = require_session(&ctx)?;
        let memberships = self
            .state
            .repo
            .list_billing_accounts_for_user(&session.user)
            .await?;
        let scope_ids: Vec<&str> = memberships.iter().map(|(b, _)| b.id.as_str()).collect();
        let mut sso_by_id = self
            .state
            .repo
            .list_sso_configs_by_scope_ids(ScopeKind::Billing, &scope_ids)
            .await?;
        let accounts: Vec<BillingAccountPb> = memberships
            .into_iter()
            .map(|(b, _)| {
                let sso = sso_by_id.remove(b.id.as_str());
                billing_to_pb_with_sso(b, sso)
            })
            .collect();
        Response::ok(ListBillingAccountsResponse {
            accounts,
            ..Default::default()
        })
    }

    async fn invite_member(
        &self,
        ctx: RequestContext,
        request: OwnedInviteMemberRequestView,
    ) -> ServiceResult<InviteMemberResponse> {
        let session = require_session(&ctx)?;
        let billing_id = BillingAccountId::from(request.billing_account_id);
        require_billing_owner(&self.state, &session, &billing_id).await?;
        validate_email(request.email)?;
        let role = role_from_i32(request.role.to_i32())?;
        let now_ms = self.state.clock.now_ms();
        let now_unix = self.state.clock.now_unix_seconds();

        if let Some(target_user) = self.state.repo.get_user_by_email(request.email).await?
            && self
                .state
                .repo
                .get_billing_membership(&target_user.id, &billing_id)
                .await?
                .is_some()
        {
            return Err(ConnectError::failed_precondition(
                "user is already a member of this billing account",
            ));
        }

        // Freeze the IdP at issuance time if billing-level requires SSO.
        let required_idp = resolve_required_sso(&self.state, &billing_id, None)
            .await?
            .filter(|c| c.required)
            .map(|c| c.idp_id);
        let invitation_id = InvitationId::new();
        let nonce = format!("inv_{}", Uuid::new_v4());
        let expires_at_ms = now_ms + self.state.config.invitation_ttl_seconds * 1000;

        // Re-invite case: an existing pending invitation gets its token
        // refreshed in place rather than throwing AlreadyExists.
        let existing = self
            .state
            .repo
            .list_pending_invitation_by_scope_email(
                ScopeKind::Billing,
                billing_id.as_str(),
                request.email,
            )
            .await?;

        let invitation = match existing {
            Some(mut inv) => {
                self.state
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
                    scope_kind: ScopeKind::Billing,
                    scope_id: billing_id.to_string(),
                    email: request.email.to_owned(),
                    role,
                    inviter_user_id: Some(session.user.clone()),
                    required_idp: required_idp.clone(),
                    nonce: nonce.clone(),
                    expires_at_ms,
                    status: InvitationStatus::Pending,
                    created_at_ms: now_ms,
                };
                self.state.repo.create_invitation(inv.clone()).await?;
                inv
            }
        };

        let token = build_invitation_token(
            &self.state,
            &invitation.id,
            &invitation.email,
            ScopeKind::Billing,
            billing_id.as_str(),
            role,
            required_idp.as_deref(),
            &nonce,
            now_unix,
        )?;

        Response::ok(InviteMemberResponse {
            invitation_id: invitation.id.to_string(),
            invite_token_for_demo: token,
            ..Default::default()
        })
    }

    async fn remove_member(
        &self,
        ctx: RequestContext,
        request: OwnedRemoveMemberRequestView,
    ) -> ServiceResult<RemoveMemberResponse> {
        let session = require_session(&ctx)?;
        let billing_id = BillingAccountId::from(request.billing_account_id);
        require_billing_owner(&self.state, &session, &billing_id).await?;
        let target_user = UserId::from(request.user_id);
        // Refuse removing the personal-billing owner — that path is
        // delete-user.
        let billing = self
            .state
            .repo
            .get_billing_account(&billing_id)
            .await?
            .ok_or_else(|| ConnectError::not_found("billing account not found"))?;
        if billing.personal {
            return Err(ConnectError::failed_precondition(
                "cannot modify membership on a personal billing account",
            ));
        }
        self.state
            .repo
            .remove_billing_membership(&target_user, &billing_id)
            .await?;
        Response::ok(RemoveMemberResponse::default())
    }

    async fn update_member_role(
        &self,
        ctx: RequestContext,
        request: OwnedUpdateMemberRoleRequestView,
    ) -> ServiceResult<UpdateMemberRoleResponse> {
        let session = require_session(&ctx)?;
        let billing_id = BillingAccountId::from(request.billing_account_id);
        require_billing_owner(&self.state, &session, &billing_id).await?;
        let target_user = UserId::from(request.user_id);
        let role = role_from_i32(request.role.to_i32())?;
        self.state
            .repo
            .update_billing_membership_role(&target_user, &billing_id, role)
            .await?;
        Response::ok(UpdateMemberRoleResponse::default())
    }

    async fn set_payment_method(
        &self,
        ctx: RequestContext,
        request: OwnedSetPaymentMethodRequestView,
    ) -> ServiceResult<SetPaymentMethodResponse> {
        let session = require_session(&ctx)?;
        let billing_id = BillingAccountId::from(request.billing_account_id);
        require_billing_owner(&self.state, &session, &billing_id).await?;
        let now_ms = self.state.clock.now_ms();
        self.state
            .billing
            .set_payment_method(&billing_id, request.payment_method_token.to_owned(), now_ms)
            .await?;
        Response::ok(SetPaymentMethodResponse::default())
    }

    async fn get_subscription(
        &self,
        ctx: RequestContext,
        request: OwnedGetSubscriptionRequestView,
    ) -> ServiceResult<GetSubscriptionResponse> {
        let session = require_session(&ctx)?;
        let billing_id = BillingAccountId::from(request.billing_account_id);
        require_billing_member(&self.state, &session, &billing_id).await?;
        let now_ms = self.state.clock.now_ms();
        self.state
            .billing
            .ensure_subscription(&billing_id, now_ms)
            .await?;
        let sub = self.state.billing.get_subscription(&billing_id).await?;
        Response::ok(GetSubscriptionResponse {
            subscription: buffa::MessageField::some(subscription_to_pb(sub)),
            ..Default::default()
        })
    }

    async fn list_invoices(
        &self,
        ctx: RequestContext,
        request: OwnedListInvoicesRequestView,
    ) -> ServiceResult<ListInvoicesResponse> {
        let session = require_session(&ctx)?;
        let billing_id = BillingAccountId::from(request.billing_account_id);
        require_billing_member(&self.state, &session, &billing_id).await?;
        let invoices = self.state.billing.list_invoices(&billing_id).await?;
        Response::ok(ListInvoicesResponse {
            invoices: invoices
                .into_iter()
                .map(|i| InvoicePb {
                    id: i.id.to_string(),
                    billing_account_id: i.billing_account_id.to_string(),
                    amount_cents: i.amount_cents,
                    currency: i.currency,
                    status: invoice_status_to_pb(&i.status),
                    issued_at: crate::services::common::ms_to_timestamp(i.issued_at_ms),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        })
    }

    async fn cancel_subscription(
        &self,
        ctx: RequestContext,
        request: OwnedCancelSubscriptionRequestView,
    ) -> ServiceResult<CancelSubscriptionResponse> {
        let session = require_session(&ctx)?;
        let billing_id = BillingAccountId::from(request.billing_account_id);
        require_billing_owner(&self.state, &session, &billing_id).await?;
        let now_ms = self.state.clock.now_ms();
        let sub = self
            .state
            .billing
            .cancel_subscription(&billing_id, now_ms)
            .await?;
        Response::ok(CancelSubscriptionResponse {
            subscription: buffa::MessageField::some(subscription_to_pb(sub)),
            ..Default::default()
        })
    }

    async fn configure_sso(
        &self,
        ctx: RequestContext,
        request: OwnedConfigureSsoRequestView,
    ) -> ServiceResult<ConfigureSsoResponse> {
        let session = require_session(&ctx)?;
        let billing_id = BillingAccountId::from(request.billing_account_id);
        require_billing_owner(&self.state, &session, &billing_id).await?;
        let now_ms = self.state.clock.now_ms();
        let cfg = SsoConfig {
            scope_kind: ScopeKind::Billing,
            scope_id: billing_id.to_string(),
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

    async fn set_auto_join_domain(
        &self,
        ctx: RequestContext,
        request: OwnedSetAutoJoinDomainRequestView,
    ) -> ServiceResult<SetAutoJoinDomainResponse> {
        let session = require_session(&ctx)?;
        let billing_id = BillingAccountId::from(request.billing_account_id);
        require_billing_owner(&self.state, &session, &billing_id).await?;
        let billing = self
            .state
            .repo
            .get_billing_account(&billing_id)
            .await?
            .ok_or_else(|| ConnectError::not_found("billing account not found"))?;
        if billing.personal {
            return Err(ConnectError::failed_precondition(
                "auto_join_domain is not supported on personal billing accounts",
            ));
        }
        let domain = if request.domain.is_empty() {
            None
        } else {
            Some(request.domain.to_owned())
        };
        self.state
            .repo
            .set_auto_join_domain(&billing_id, domain)
            .await?;
        Response::ok(SetAutoJoinDomainResponse::default())
    }

    async fn delete_billing_account(
        &self,
        ctx: RequestContext,
        request: OwnedDeleteBillingAccountRequestView,
    ) -> ServiceResult<DeleteBillingAccountResponse> {
        let session = require_session(&ctx)?;
        let billing_id = BillingAccountId::from(request.billing_account_id);
        require_billing_owner(&self.state, &session, &billing_id).await?;
        self.state.repo.delete_billing_account(&billing_id).await?;
        Response::ok(DeleteBillingAccountResponse::default())
    }
}
