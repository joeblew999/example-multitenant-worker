//! Multi-tenant SaaS auth on Cloudflare Workers.
//!
//! The fetch handler pulls in env config, builds a per-request
//! `AppState`, registers the four ConnectRPC services, layers
//! `AuthLayer` (verifies the session macaroon if present) on top of
//! `RequestIdLayer` (per-request trace id), and dispatches.

#![allow(refining_impl_trait)]

use std::sync::Arc;

use connectrpc::{ConnectRpcBody, ConnectRpcService, Router as RpcRouter};
use tower::{Layer, Service};
use worker::{Context, Env, HttpRequest, event};

pub(crate) mod proto {
    connectrpc::include_generated!();
}

pub mod auth;
pub mod billing;
pub mod domain;
pub mod middleware;
pub mod observability;
pub mod routes;
pub mod services;
pub mod state;
pub mod store;
pub mod time;

use crate::auth::Keyring;
use crate::billing::BillingProvider;
use crate::middleware::{
    AuthLayer, build_authorizer, metrics_layer, rate_limit_observe_layer, shadow_layer,
    tracing_layer,
};
use crate::proto::workers::auth::v1::AuthServiceExt;
use crate::proto::workers::billing::v1::BillingServiceExt;
use crate::proto::workers::invitation::v1::InvitationServiceExt;
use crate::proto::workers::org::v1::OrgServiceExt;
use crate::services::{AuthServer, BillingServer, InvitationServer, OrgServer};
use crate::state::{AppState, Config, SharedState};
use crate::store::Repo;
use crate::time::{SharedClock, SystemClock};

#[event(fetch, respond_with_errors)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> worker::Result<http::Response<ConnectRpcBody>> {
    // Install tracing-subscriber → JS console bridge. Idempotent — first
    // call wins, subsequent ones no-op. Without this, every
    // `tracing::info!` / `warn!` from the middleware family is dropped
    // silently. See observability.rs.
    observability::init();

    if let Some(resp) = routes::try_handle(&req) {
        return Ok(resp);
    }

    let state = build_state(&env).await?;
    let auth_layer = AuthLayer::new(Arc::clone(&state.keyring), Arc::clone(&state.clock));

    // CedarLayer in shadow mode: evaluates every request, logs the
    // decision, never rejects. The hand-rolled `services::authz::require_*`
    // helpers still enforce — they're the source of truth until shadow
    // mode runs cleanly in prod for N days. See KUMO.md style of
    // separation between the two layers in CLAUDE.md and the rollout
    // plan in examples/multitenant-policies/ROADMAP.md.
    let cedar_authorizer = build_authorizer();
    let cedar_layer = shadow_layer::<worker::Body>(Arc::clone(&cedar_authorizer));

    // -----------------------------------------------------------------
    // CF observability + abuse stack (outside-in):
    //
    //   cf_tracing       — outermost; span wraps everything below so
    //                      every event downstream carries cf.* fields
    //   cf_metrics       — counter + latency for the full request
    //                      including all middleware overhead, written
    //                      to Analytics Engine
    //   cf_rate_limit    — rejects abuse BEFORE any auth/DB work
    //                      (per-IP via cf-connecting-ip; Mode::Observe
    //                      until logs confirm key derivation)
    //   auth_layer       — extracts session if present
    //   cedar_layer      — path-based authz (Mode::Shadow)
    //   ConnectRpcService — innermost; the actual dispatch
    //
    // Order matters: rate-limit on TOP of auth so we can throttle
    // anonymous floods cheaply. Metrics on top of rate-limit so 429s
    // get counted (status_class=4xx). Tracing on top of everything so
    // its span context flows downward into rate-limit's warn logs and
    // cedar's decision logs.
    //
    // Each binding only present on wasm32 (the `Env` shape differs in
    // native tests — see build_state's cfg-gated arms). For native
    // unit tests we just hit ConnectRpcService directly via the
    // handlers, not through this stack.
    // -----------------------------------------------------------------
    let cf_tracing = tracing_layer::<worker::Body>();
    let cf_metrics = metrics_layer(env.analytics_engine("AE")?);
    let cf_rate_limit = rate_limit_observe_layer::<worker::Body>(env.get_binding("RL")?);

    let router = RpcRouter::new();
    let router = register_services(router, &state);

    let mut svc = cf_tracing.layer(
        cf_metrics.layer(
            cf_rate_limit.layer(
                auth_layer.layer(cedar_layer.layer(ConnectRpcService::new(router))),
            ),
        ),
    );
    svc.call(req)
        .await
        .map_err(|e| worker::Error::RustError(format!("rpc dispatch: {e}")))
}

fn register_services<R: Repo, B: BillingProvider>(
    router: RpcRouter,
    state: &SharedState<R, B>,
) -> RpcRouter {
    let router = Arc::new(AuthServer::new(Arc::clone(state))).register(router);
    let router = Arc::new(BillingServer::new(Arc::clone(state))).register(router);
    let router = Arc::new(OrgServer::new(Arc::clone(state))).register(router);
    Arc::new(InvitationServer::new(Arc::clone(state))).register(router)
}

#[cfg(target_arch = "wasm32")]
async fn build_state(
    env: &Env,
) -> worker::Result<SharedState<store::D1Repo, billing::D1BillingProvider>> {
    let db = env.d1("DB")?;
    let repo = store::D1Repo::new(db);
    let billing = billing::D1BillingProvider::new(env.d1("DB")?);

    let keyring = Arc::new(load_keyring(env));
    let clock: SharedClock = Arc::new(SystemClock::new());
    let config = load_config(env);

    Ok(Arc::new(AppState {
        repo,
        billing,
        keyring,
        clock,
        config,
    }))
}

#[cfg(not(target_arch = "wasm32"))]
async fn build_state(
    _env: &Env,
) -> worker::Result<SharedState<store::InMemoryRepo, billing::InMemoryBillingProvider>> {
    let keyring = Arc::new(Keyring::dev_default());
    let clock: SharedClock = Arc::new(SystemClock::new());
    let config = Config::default();
    Ok(Arc::new(AppState {
        repo: store::InMemoryRepo::new(),
        billing: billing::InMemoryBillingProvider::new(),
        keyring,
        clock,
        config,
    }))
}

#[cfg(target_arch = "wasm32")]
fn load_keyring(env: &Env) -> Keyring {
    let raw = env
        .var("SESSION_KEY")
        .ok()
        .map(|v| v.to_string())
        .unwrap_or_default();
    if raw.trim().is_empty() {
        return Keyring::dev_default();
    }
    Keyring::from_base64(&raw)
        .expect("SESSION_KEY is set but invalid — refusing to start with a broken key")
}

#[cfg(target_arch = "wasm32")]
fn load_config(env: &Env) -> Config {
    let read_secs = |k: &str, default: i64| {
        env.var(k)
            .ok()
            .and_then(|v| v.to_string().parse::<i64>().ok())
            .unwrap_or(default)
    };
    let read_bool = |k: &str, default: bool| {
        env.var(k)
            .ok()
            .map(|v| {
                let s = v.to_string();
                matches!(s.as_str(), "true" | "1" | "yes")
            })
            .unwrap_or(default)
    };
    Config {
        session_ttl_seconds: read_secs("SESSION_TTL_SECONDS", 86_400),
        invitation_ttl_seconds: read_secs("INVITATION_TTL_SECONDS", 7 * 86_400),
        password_reset_ttl_seconds: read_secs("PASSWORD_RESET_TTL_SECONDS", 15 * 60),
        email_verify_ttl_seconds: read_secs("EMAIL_VERIFY_TTL_SECONDS", 86_400),
        sso_state_ttl_seconds: read_secs("SSO_STATE_TTL_SECONDS", 10 * 60),
        enforce_email_verification: read_bool("ENFORCE_EMAIL_VERIFICATION", false),
    }
}

#[cfg(test)]
mod tests {
    //! High-level handler integration tests against the in-memory backing
    //! store. These exercise the same handler code that runs against D1
    //! under wasm.

    use std::sync::Arc;

    use buffa::view::OwnedView;
    use connectrpc::RequestContext as RpcContext;
    use futures::executor::block_on;

    use crate::auth::Keyring;
    use crate::billing::InMemoryBillingProvider;
    use crate::domain::{AuthMethod, Role};
    use crate::proto::workers::auth::v1::{
        AuthService, ChangePasswordRequest, LoginRequest, SignupRequest, SwitchContextRequest,
        WhoamiRequest,
    };
    use crate::proto::workers::billing::v1::{BillingService, InviteMemberRequest};
    use crate::proto::workers::org::v1::{CreateOrganizationRequest, OrgService};
    use crate::services::{AuthServer, BillingServer, OrgServer};
    use crate::state::{AppState, Config, SharedState};
    use crate::store::{BillingRepo, InMemoryRepo, MembershipRepo};
    use crate::time::{SharedClock, SystemClock};

    type TestRepo = InMemoryRepo;
    type TestBilling = InMemoryBillingProvider;

    fn build_test_state() -> SharedState<TestRepo, TestBilling> {
        let keyring = Arc::new(Keyring::dev_default());
        let clock: SharedClock = Arc::new(SystemClock::new());
        Arc::new(AppState {
            repo: InMemoryRepo::new(),
            billing: InMemoryBillingProvider::new(),
            keyring,
            clock,
            config: Config::default(),
        })
    }

    fn ctx_with_session(session: crate::auth::SessionContext) -> RpcContext {
        let mut ctx = RpcContext::default();
        ctx.extensions.insert(session);
        ctx
    }

    fn view<V>(msg: &V::Owned) -> OwnedView<V>
    where
        V: buffa::MessageView<'static>,
        V::Owned: buffa::Message,
    {
        OwnedView::<V>::from_owned(msg).expect("build request view")
    }

    fn signup(server: &AuthServer<TestRepo, TestBilling>, email: &str, password: &str) -> String {
        let req = SignupRequest {
            email: email.into(),
            password: password.into(),
            invite_token: None,
            ..Default::default()
        };
        let resp = block_on(server.signup(RpcContext::default(), view(&req)))
            .unwrap()
            .body;
        resp.session_token
    }

    fn parse_session(token: &str, kr: &Keyring) -> crate::auth::SessionContext {
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        crate::auth::verify_session(kr, now_unix, token).unwrap()
    }

    #[test]
    fn signup_then_login_round_trip() {
        let state = build_test_state();
        let server = AuthServer::new(Arc::clone(&state));
        let token = signup(&server, "ada@example.com", "correct horse staple");
        // Login with same credentials.
        let req = LoginRequest {
            email: "ada@example.com".into(),
            password: "correct horse staple".into(),
            ..Default::default()
        };
        let resp = block_on(server.login(RpcContext::default(), view(&req)))
            .unwrap()
            .body;
        assert!(!resp.session_token.is_empty());
        let session = parse_session(&token, &state.keyring);
        assert_eq!(session.email, "ada@example.com");
        assert_eq!(session.role, Role::Owner);
        assert_eq!(session.auth_method, AuthMethod::Password);
    }

    #[test]
    fn login_rejects_wrong_password() {
        let state = build_test_state();
        let server = AuthServer::new(Arc::clone(&state));
        let _ = signup(&server, "x@y", "right-password");
        let req = LoginRequest {
            email: "x@y".into(),
            password: "wrong-password".into(),
            ..Default::default()
        };
        let err = block_on(server.login(RpcContext::default(), view(&req))).unwrap_err();
        assert_eq!(err.code, connectrpc::ErrorCode::Unauthenticated);
    }

    #[test]
    fn whoami_lists_billing_memberships() {
        let state = build_test_state();
        let auth = AuthServer::new(Arc::clone(&state));
        let token = signup(&auth, "ada@example.com", "verylong-password");
        let session = parse_session(&token, &state.keyring);
        let ctx = ctx_with_session(session);
        let resp = block_on(auth.whoami(ctx, view(&WhoamiRequest::default())))
            .unwrap()
            .body;
        let info = resp.whoami.into_option().unwrap();
        assert_eq!(info.email, "ada@example.com");
        assert_eq!(info.billing_memberships.len(), 1);
        assert!(
            info.billing_memberships[0]
                .display_name
                .contains("personal")
        );
    }

    #[test]
    fn invite_accept_via_signup_creates_billing_membership() {
        let state = build_test_state();
        let auth = AuthServer::new(Arc::clone(&state));
        let billing_server = BillingServer::new(Arc::clone(&state));

        // Owner signs up and creates a non-personal billing account.
        let owner_token = signup(&auth, "owner@example.com", "verylong-password");
        let owner_session = parse_session(&owner_token, &state.keyring);
        let owner_user = owner_session.user.clone();

        // Need a non-personal billing for the invite. Create one
        // directly via the repo (no public RPC for this in the demo).
        let billing_acc = block_on(state.repo.create_billing_account(
            "Acme".into(),
            owner_user.clone(),
            state.clock.now_ms(),
        ))
        .unwrap();

        // Switch context to operate from Acme.
        let switch_req = SwitchContextRequest {
            billing_account_id: Some(billing_acc.id.to_string()),
            org_id: None,
            ..Default::default()
        };
        let switched = block_on(
            auth.switch_context(ctx_with_session(owner_session.clone()), view(&switch_req)),
        )
        .unwrap()
        .body;
        let owner_session_in_acme = parse_session(&switched.session_token, &state.keyring);

        // Invite a new user.
        let invite_req = InviteMemberRequest {
            billing_account_id: billing_acc.id.to_string(),
            email: "newcomer@example.com".into(),
            role: crate::proto::workers::billing::v1::Role::ROLE_MEMBER.into(),
            ..Default::default()
        };
        let invite_resp = block_on(billing_server.invite_member(
            ctx_with_session(owner_session_in_acme.clone()),
            view(&invite_req),
        ))
        .unwrap()
        .body;
        assert!(!invite_resp.invite_token_for_demo.is_empty());

        // Newcomer signs up with the invite token.
        let signup_req = SignupRequest {
            email: "newcomer@example.com".into(),
            password: "another-very-long-password".into(),
            invite_token: Some(invite_resp.invite_token_for_demo.clone()),
            ..Default::default()
        };
        let signup_resp = block_on(auth.signup(RpcContext::default(), view(&signup_req)))
            .unwrap()
            .body;
        let newcomer_session = parse_session(&signup_resp.session_token, &state.keyring);

        // Membership should now exist.
        let mem = block_on(
            state
                .repo
                .get_billing_membership(&newcomer_session.user, &billing_acc.id),
        )
        .unwrap();
        assert!(mem.is_some());
        assert_eq!(mem.unwrap().role, Role::Member);
    }

    #[test]
    fn create_org_and_member_visibility() {
        let state = build_test_state();
        let auth = AuthServer::new(Arc::clone(&state));
        let org_server = OrgServer::new(Arc::clone(&state));

        let owner_token = signup(&auth, "owner@example.com", "verylong-password");
        let owner_session = parse_session(&owner_token, &state.keyring);
        let owner_user = owner_session.user.clone();
        let billing = block_on(state.repo.create_billing_account(
            "Acme".into(),
            owner_user.clone(),
            state.clock.now_ms(),
        ))
        .unwrap();

        // Switch to Acme, then create an org.
        let switch_req = SwitchContextRequest {
            billing_account_id: Some(billing.id.to_string()),
            org_id: None,
            ..Default::default()
        };
        let switched =
            block_on(auth.switch_context(ctx_with_session(owner_session), view(&switch_req)))
                .unwrap()
                .body;
        let owner_in_acme = parse_session(&switched.session_token, &state.keyring);

        let org_req = CreateOrganizationRequest {
            billing_account_id: billing.id.to_string(),
            display_name: "Engineering".into(),
            ..Default::default()
        };
        let org_resp = block_on(
            org_server.create_organization(ctx_with_session(owner_in_acme), view(&org_req)),
        )
        .unwrap()
        .body;
        let org = org_resp.organization.into_option().unwrap();
        assert_eq!(org.display_name, "Engineering");
        assert_eq!(org.billing_account_id, billing.id.to_string());
    }

    #[test]
    fn change_password_requires_old_password() {
        let state = build_test_state();
        let auth = AuthServer::new(Arc::clone(&state));
        let token = signup(&auth, "x@y", "old-very-long-password");
        let session = parse_session(&token, &state.keyring);
        let req = ChangePasswordRequest {
            old_password: "wrong-old".into(),
            new_password: "another-very-long-pw".into(),
            ..Default::default()
        };
        let err =
            block_on(auth.change_password(ctx_with_session(session), view(&req))).unwrap_err();
        assert_eq!(err.code, connectrpc::ErrorCode::Unauthenticated);
    }
}
