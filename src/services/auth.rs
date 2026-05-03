//! `workers.auth.v1.AuthService` implementation.
//!
//! Covers signup / login / SSO / link-identity / password-change /
//! password-reset / context-switch / whoami. Sessions are minted with
//! caveats matching the plan: `purpose=session`, `user`, `email`,
//! `billing`, `role`, `auth_method`, `exp`, optional `org`. Verification
//! is pure (no DB).

#![allow(clippy::too_many_arguments)]

use connectrpc::{ConnectError, RequestContext, Response, ServiceResult};
use uuid::Uuid;

use crate::auth::{
    MintInvitationInput, MintSessionInput, hash_new_password, mint_invitation, mint_password_reset,
    mint_session, verify_invitation, verify_password, verify_password_reset,
};
use crate::billing::BillingProvider;
use crate::domain::{
    AuthMethod, BillingAccountId, IdentityProvider, OrgId, Role, ScopeKind, ScopeTarget, SsoState,
    TokenPurpose,
};
use crate::middleware::require_session;
use crate::proto::workers::auth::v1::{
    AuthService, ChangePasswordResponse, ConsumePasswordResetResponse, LinkIdentityResponse,
    LoginResponse, OwnedChangePasswordRequestView, OwnedConsumePasswordResetRequestView,
    OwnedLinkIdentityRequestView, OwnedLoginRequestView, OwnedRequestPasswordResetRequestView,
    OwnedSignupRequestView, OwnedSsoCompleteRequestView, OwnedSsoStartRequestView,
    OwnedSwitchContextRequestView, OwnedWhoamiRequestView, RequestPasswordResetResponse,
    SignupResponse, SsoCompleteResponse, SsoStartResponse, SwitchContextResponse, WhoamiResponse,
};
use crate::services::common::{
    build_whoami, build_whoami_async, email_domain, fetch_membership_lists,
    fetch_password_identity, fetch_user, find_personal_billing, issue_personal_session,
    map_already_exists_as_precondition, map_password_error, map_token_error, parse_scope_kind,
    resolve_required_sso, validate_email, validate_pending_invitation,
};
use crate::state::SharedState;
use crate::store::{InvitationAcceptance, NewPasswordUser, NewSsoUser, Repo};

pub struct AuthServer<R: Repo, B: BillingProvider> {
    state: SharedState<R, B>,
}

impl<R: Repo, B: BillingProvider> AuthServer<R, B> {
    pub fn new(state: SharedState<R, B>) -> Self {
        Self { state }
    }
}

impl<R: Repo, B: BillingProvider> AuthService for AuthServer<R, B> {
    async fn signup(
        &self,
        _ctx: RequestContext,
        request: OwnedSignupRequestView,
    ) -> ServiceResult<SignupResponse> {
        validate_email(request.email)?;
        let now_ms = self.state.clock.now_ms();
        let now_unix = self.state.clock.now_unix_seconds();

        let invite_acceptance: Option<InvitationAcceptance> = match request.invite_token {
            Some(token) => {
                let parsed = verify_invitation(&self.state.keyring, now_unix, token)
                    .map_err(map_token_error)?;
                if !parsed.email.eq_ignore_ascii_case(request.email) {
                    return Err(ConnectError::permission_denied(
                        "invite email doesn't match signup email",
                    ));
                }
                let stored = self
                    .state
                    .repo
                    .get_invitation(&parsed.invitation_id)
                    .await?
                    .ok_or_else(|| ConnectError::not_found("invitation no longer exists"))?;
                validate_pending_invitation(&stored, &parsed.nonce)?;
                Some(InvitationAcceptance {
                    invitation_id: parsed.invitation_id,
                    target: ScopeTarget::from_parts(parsed.scope_kind, parsed.scope_id),
                    role: parsed.role,
                })
            }
            None => None,
        };

        let password_hash = hash_new_password(request.password)
            .await
            .map_err(map_password_error)?;

        let user = self
            .state
            .repo
            .create_password_user(
                NewPasswordUser {
                    email: request.email.to_owned(),
                    password_hash,
                },
                invite_acceptance,
                now_ms,
            )
            .await?;

        let (token, whoami) =
            issue_personal_session(&self.state, &user, AuthMethod::Password).await?;

        Response::ok(SignupResponse {
            session_token: token,
            whoami: buffa::MessageField::some(whoami),
            ..Default::default()
        })
    }

    async fn login(
        &self,
        _ctx: RequestContext,
        request: OwnedLoginRequestView,
    ) -> ServiceResult<LoginResponse> {
        let user = self
            .state
            .repo
            .get_user_by_email(request.email)
            .await?
            .ok_or_else(|| ConnectError::unauthenticated("invalid credentials"))?;

        let billings = self
            .state
            .repo
            .list_billing_accounts_for_user(&user.id)
            .await?;
        let personal = find_personal_billing(&billings)?;

        // Check SSO-required before paying Argon2 cost — otherwise a
        // bot hammering an SSO-locked account drains worker CPU.
        if let Some(sso) = resolve_required_sso(&self.state, &personal.id, None).await?
            && sso.required
            && sso.break_glass_user_id != user.id
        {
            return Err(ConnectError::permission_denied(
                "SSO is required on this account; password login is disabled",
            ));
        }

        let password_identity = fetch_password_identity(&self.state, &user.id, || {
            ConnectError::unauthenticated(
                "this account doesn't have a password identity; sign in via SSO",
            )
        })
        .await?;
        let stored = password_identity
            .secret
            .as_deref()
            .ok_or_else(|| ConnectError::internal("password identity missing stored hash"))?;
        verify_password(stored, request.password).map_err(map_password_error)?;

        let auth_method = AuthMethod::Password;
        let now_unix = self.state.clock.now_unix_seconds();
        let token = mint_session(
            &self.state.keyring,
            MintSessionInput {
                user_id: &user.id,
                email: &user.email,
                billing: &personal.id,
                org: None,
                role: Role::Owner,
                auth_method: &auth_method,
                now_unix,
                ttl_seconds: self.state.config.session_ttl_seconds,
            },
        )
        .map_err(map_token_error)?;

        let orgs = self
            .state
            .repo
            .list_organizations_for_user(&user.id)
            .await?;
        let whoami = build_whoami(
            &user,
            &personal,
            None,
            Role::Owner,
            &auth_method,
            billings,
            orgs,
        );

        Response::ok(LoginResponse {
            session_token: token,
            whoami: buffa::MessageField::some(whoami),
            ..Default::default()
        })
    }

    async fn sso_start(
        &self,
        _ctx: RequestContext,
        request: OwnedSsoStartRequestView,
    ) -> ServiceResult<SsoStartResponse> {
        let now_ms = self.state.clock.now_ms();
        let scope_hint = request.scope_hint;
        let (scope_kind, scope_id) = parse_sso_scope(scope_hint)?;
        let cfg = match scope_kind {
            ScopeKind::Org => {
                let org_id = OrgId::from(scope_id.as_str());
                match self.state.repo.get_organization(&org_id).await? {
                    Some(org) => {
                        resolve_required_sso(&self.state, &org.billing_account_id, Some(&org_id))
                            .await?
                    }
                    None => None,
                }
            }
            ScopeKind::Billing => {
                let billing_id = BillingAccountId::from(scope_id.as_str());
                resolve_required_sso(&self.state, &billing_id, None).await?
            }
        }
        .ok_or_else(|| ConnectError::not_found("no SSO configured for this scope"))?;

        let state = format!("ssost_{}", Uuid::new_v4());
        let expires_at_ms = now_ms + self.state.config.sso_state_ttl_seconds * 1000;
        self.state
            .repo
            .store_sso_state(SsoState {
                state: state.clone(),
                scope_hint: scope_hint.to_owned(),
                expected_idp: Some(cfg.idp_id.clone()),
                created_at_ms: now_ms,
                expires_at_ms,
            })
            .await?;
        let sep = if cfg.login_url.contains('?') {
            '&'
        } else {
            '?'
        };
        let redirect_url = format!("{}{}state={}", cfg.login_url, sep, state);
        Response::ok(SsoStartResponse {
            redirect_url,
            state,
            ..Default::default()
        })
    }

    async fn sso_complete(
        &self,
        _ctx: RequestContext,
        request: OwnedSsoCompleteRequestView,
    ) -> ServiceResult<SsoCompleteResponse> {
        validate_email(request.email)?;
        let now_ms = self.state.clock.now_ms();
        let state = self
            .state
            .repo
            .consume_sso_state(request.state, now_ms)
            .await?
            .ok_or_else(|| ConnectError::invalid_argument("unknown or expired SSO state"))?;
        if let Some(expected) = state.expected_idp.as_deref()
            && expected != request.idp_id
        {
            return Err(ConnectError::permission_denied(
                "SSO state was issued for a different IdP",
            ));
        }

        let provider = IdentityProvider::sso_str(request.idp_id);
        let mut jit_created = false;
        let user = if let Some(identity) = self
            .state
            .repo
            .find_identity(&provider, request.idp_user_id)
            .await?
        {
            self.state
                .repo
                .get_user(&identity.user_id)
                .await?
                .ok_or_else(|| ConnectError::internal("identity references missing user"))?
        } else if let Some(user) = self.state.repo.get_user_by_email(request.email).await? {
            self.state
                .repo
                .link_identity(&user.id, &provider, request.idp_user_id, None, now_ms)
                .await?;
            user
        } else {
            jit_created = true;
            let auto_join = match email_domain(request.email) {
                Some(domain) => {
                    self.state
                        .repo
                        .find_billing_by_auto_join_domain(&domain)
                        .await?
                }
                None => None,
            };
            let auto_join_id = auto_join.map(|b| b.id);
            self.state
                .repo
                .create_sso_user(
                    NewSsoUser {
                        email: request.email.to_owned(),
                        idp_id: request.idp_id.to_owned(),
                        idp_user_id: request.idp_user_id.to_owned(),
                    },
                    None,
                    auto_join_id,
                    now_ms,
                )
                .await?
        };

        let auth_method = AuthMethod::Sso(request.idp_id.to_owned());
        let (token, whoami) = issue_personal_session(&self.state, &user, auth_method).await?;
        Response::ok(SsoCompleteResponse {
            session_token: token,
            whoami: buffa::MessageField::some(whoami),
            jit_created,
            ..Default::default()
        })
    }

    async fn link_identity(
        &self,
        ctx: RequestContext,
        request: OwnedLinkIdentityRequestView,
    ) -> ServiceResult<LinkIdentityResponse> {
        let session = require_session(&ctx)?;
        let now_ms = self.state.clock.now_ms();
        let provider = IdentityProvider::decode(request.provider).ok_or_else(|| {
            ConnectError::invalid_argument(format!("unknown provider {}", request.provider))
        })?;
        // For the demo we treat `code` as the IdP-side identifier verbatim.
        // Production would exchange the OAuth code with the IdP first.
        let provider_user_id = request.code;
        let identity = self
            .state
            .repo
            .link_identity(
                &session.user,
                &provider.encode(),
                provider_user_id,
                None,
                now_ms,
            )
            .await?;
        Response::ok(LinkIdentityResponse {
            identity_id: identity.id.to_string(),
            ..Default::default()
        })
    }

    async fn change_password(
        &self,
        ctx: RequestContext,
        request: OwnedChangePasswordRequestView,
    ) -> ServiceResult<ChangePasswordResponse> {
        let session = require_session(&ctx)?;
        let password_identity = fetch_password_identity(&self.state, &session.user, || {
            ConnectError::failed_precondition(
                "this account doesn't have a password identity to change",
            )
        })
        .await?;
        let stored = password_identity
            .secret
            .as_deref()
            .ok_or_else(|| ConnectError::internal("password identity missing stored hash"))?;
        verify_password(stored, request.old_password).map_err(map_password_error)?;
        let new_hash = hash_new_password(request.new_password)
            .await
            .map_err(map_password_error)?;
        self.state
            .repo
            .update_identity_secret(&password_identity.id, new_hash)
            .await?;
        Response::ok(ChangePasswordResponse::default())
    }

    async fn request_password_reset(
        &self,
        _ctx: RequestContext,
        request: OwnedRequestPasswordResetRequestView,
    ) -> ServiceResult<RequestPasswordResetResponse> {
        let now_unix = self.state.clock.now_unix_seconds();
        let user = self.state.repo.get_user_by_email(request.email).await?;
        let token = match user {
            Some(user) => {
                let nonce = format!("pwr_{}", Uuid::new_v4());
                let token = mint_password_reset(
                    &self.state.keyring,
                    &user.id,
                    &nonce,
                    now_unix,
                    self.state.config.password_reset_ttl_seconds,
                )
                .map_err(map_token_error)?;
                Some(token)
            }
            None => None,
        };
        Response::ok(RequestPasswordResetResponse {
            reset_token_for_demo: token,
            ..Default::default()
        })
    }

    async fn consume_password_reset(
        &self,
        _ctx: RequestContext,
        request: OwnedConsumePasswordResetRequestView,
    ) -> ServiceResult<ConsumePasswordResetResponse> {
        let now_ms = self.state.clock.now_ms();
        let now_unix = self.state.clock.now_unix_seconds();
        let parsed = verify_password_reset(&self.state.keyring, now_unix, request.token)
            .map_err(map_token_error)?;
        // Stamp the nonce first so concurrent uses can't double-spend.
        self.state
            .repo
            .consume_nonce(&parsed.nonce, TokenPurpose::PasswordReset.as_str(), now_ms)
            .await
            .map_err(|e| map_already_exists_as_precondition(e, "reset token already used"))?;

        let password_identity = fetch_password_identity(&self.state, &parsed.user_id, || {
            ConnectError::failed_precondition(
                "this account doesn't have a password identity to reset",
            )
        })
        .await?;
        let new_hash = hash_new_password(request.new_password)
            .await
            .map_err(map_password_error)?;
        self.state
            .repo
            .update_identity_secret(&password_identity.id, new_hash)
            .await?;

        let user = fetch_user(&self.state, &parsed.user_id).await?;
        let (token, whoami) =
            issue_personal_session(&self.state, &user, AuthMethod::Password).await?;
        Response::ok(ConsumePasswordResetResponse {
            session_token: token,
            whoami: buffa::MessageField::some(whoami),
            ..Default::default()
        })
    }

    async fn switch_context(
        &self,
        ctx: RequestContext,
        request: OwnedSwitchContextRequestView,
    ) -> ServiceResult<SwitchContextResponse> {
        let session = require_session(&ctx)?;
        let now_unix = self.state.clock.now_unix_seconds();

        let user = fetch_user(&self.state, &session.user).await?;

        let target_billing_id = match request.billing_account_id {
            Some(s) if !s.is_empty() => BillingAccountId::from(s),
            _ => session.billing.clone(),
        };
        let target_org_id: Option<OrgId> = match request.org_id {
            Some(s) if !s.is_empty() => Some(OrgId::from(s)),
            _ => None,
        };

        let (billing_membership, billing) = futures::try_join!(
            self.state
                .repo
                .get_billing_membership(&user.id, &target_billing_id),
            self.state.repo.get_billing_account(&target_billing_id),
        )?;
        let billing_membership = billing_membership.ok_or_else(|| {
            ConnectError::permission_denied("not a member of the target billing account")
        })?;
        let billing =
            billing.ok_or_else(|| ConnectError::not_found("billing account no longer exists"))?;

        let sso =
            resolve_required_sso(&self.state, &target_billing_id, target_org_id.as_ref()).await?;
        if let Some(cfg) = sso
            && cfg.required
            && cfg.break_glass_user_id != user.id
        {
            let needed = AuthMethod::Sso(cfg.idp_id.clone());
            if session.auth_method != needed {
                return Err(ConnectError::permission_denied(
                    "scope requires SSO; switch via SsoStart/SsoComplete",
                ));
            }
        }

        let mut role = billing_membership.role;
        let target_org = if let Some(org_id) = &target_org_id {
            let (org, mem) = futures::try_join!(
                self.state.repo.get_organization(org_id),
                self.state.repo.get_org_membership(&user.id, org_id),
            )?;
            let org = org.ok_or_else(|| ConnectError::not_found("organization not found"))?;
            if org.billing_account_id != target_billing_id {
                return Err(ConnectError::failed_precondition(
                    "organization does not belong to the target billing account",
                ));
            }
            let mem = mem
                .ok_or_else(|| ConnectError::permission_denied("not a member of the target org"))?;
            role = mem.role;
            Some(org)
        } else {
            None
        };

        let token = mint_session(
            &self.state.keyring,
            MintSessionInput {
                user_id: &user.id,
                email: &user.email,
                billing: &target_billing_id,
                org: target_org_id.as_ref(),
                role,
                auth_method: &session.auth_method,
                now_unix,
                ttl_seconds: self.state.config.session_ttl_seconds,
            },
        )
        .map_err(map_token_error)?;

        let whoami = build_whoami_async(
            &self.state,
            &user,
            &billing,
            target_org.as_ref(),
            role,
            &session.auth_method,
        )
        .await?;
        Response::ok(SwitchContextResponse {
            session_token: token,
            whoami: buffa::MessageField::some(whoami),
            ..Default::default()
        })
    }

    async fn whoami(
        &self,
        ctx: RequestContext,
        _request: OwnedWhoamiRequestView,
    ) -> ServiceResult<WhoamiResponse> {
        let session = require_session(&ctx)?;
        let org_lookup = async {
            match &session.org {
                Some(id) => Ok(self.state.repo.get_organization(id).await?),
                None => Ok::<_, ConnectError>(None),
            }
        };
        let (user, billing, org, (billings, orgs)) = futures::try_join!(
            fetch_user(&self.state, &session.user),
            async {
                self.state
                    .repo
                    .get_billing_account(&session.billing)
                    .await?
                    .ok_or_else(|| ConnectError::not_found("session billing not found"))
            },
            org_lookup,
            fetch_membership_lists(&self.state, &session.user),
        )?;
        let whoami = build_whoami(
            &user,
            &billing,
            org.as_ref(),
            session.role,
            &session.auth_method,
            billings,
            orgs,
        );
        Response::ok(WhoamiResponse {
            whoami: buffa::MessageField::some(whoami),
            ..Default::default()
        })
    }
}

fn parse_sso_scope(s: &str) -> Result<(ScopeKind, String), ConnectError> {
    let (kind, id) = s.split_once(':').ok_or_else(|| {
        ConnectError::invalid_argument("scope_hint must be 'billing:<id>' or 'org:<id>'")
    })?;
    Ok((parse_scope_kind(kind)?, id.to_owned()))
}

pub fn build_invitation_token<R: Repo, B: BillingProvider>(
    state: &SharedState<R, B>,
    invitation_id: &crate::domain::InvitationId,
    email: &str,
    scope_kind: ScopeKind,
    scope_id: &str,
    role: Role,
    required_idp: Option<&str>,
    nonce: &str,
    now_unix: i64,
) -> Result<String, ConnectError> {
    mint_invitation(
        &state.keyring,
        MintInvitationInput {
            invitation_id,
            email,
            scope_kind,
            scope_id,
            role,
            required_idp,
            nonce,
            now_unix,
            ttl_seconds: state.config.invitation_ttl_seconds,
        },
    )
    .map_err(map_token_error)
}
