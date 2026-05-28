//! Cedar authz layer wired into this worker.
//!
//! Sits between `AuthLayer` (which inserts `SessionContext` into
//! request extensions) and the ConnectRPC service router. Runs in
//! **shadow mode**: every request is evaluated against the editorial
//! policy set, the decision is logged via `tracing`, and the layer
//! ALWAYS calls inner. The existing `services::authz::require_*`
//! helpers continue to enforce.
//!
//! After shadow mode runs cleanly in prod for N days, flip the layer's
//! constructor to `enforce()` and delete `services::authz` (per
//! `examples/multitenant-policies/ROADMAP.md` structural change A).

use std::sync::Arc;

use cedar_policy::{Context, EntityUid, RestrictedExpression};
use connectrpc_cedar::{
    CedarAuthorizer, CedarLayer, CedarRequest, action::action_from_path,
};

use crate::auth::SessionContext;
use crate::domain::Role;

const SCHEMA: &str = include_str!("../../../../examples/multitenant-policies/multitenant.cedarschema");
const BILLING_POLICIES: &str = include_str!("../../../../examples/multitenant-policies/policies/billing.cedar");
const INVITATION_POLICIES: &str = include_str!("../../../../examples/multitenant-policies/policies/invitation.cedar");
const ORG_POLICIES: &str = include_str!("../../../../examples/multitenant-policies/policies/org.cedar");

/// Build the editorial-policies authorizer. Called once at worker boot.
/// Panics on schema/policy errors — those are deploy-time bugs, not
/// runtime ones, and we want the deploy to FAIL rather than serve
/// every request with default-deny silence.
pub fn build_authorizer() -> Arc<CedarAuthorizer> {
    let combined = format!("{BILLING_POLICIES}\n\n{INVITATION_POLICIES}\n\n{ORG_POLICIES}");
    Arc::new(
        CedarAuthorizer::from_str(SCHEMA, &combined)
            .expect("editorial Cedar policies failed to validate at worker boot"),
    )
}

/// Build the CedarLayer for this worker in **shadow mode**.
///
/// `B` is the request body type (typically `worker::Body` aka
/// `http::body::Body` for wasm32, but stays generic so unit tests can
/// use a different body without redefining the extractor).
pub fn shadow_layer<B: 'static>(
    authorizer: Arc<CedarAuthorizer>,
) -> CedarLayer<
    impl Fn(&http::Request<B>) -> Option<CedarRequest> + Clone + Send + Sync + 'static,
> {
    CedarLayer::shadow(authorizer, |req: &http::Request<B>| extract_cedar_request(req))
        .skip_paths([
            // Public routes that AuthLayer doesn't gate either.
            "/healthz",
            "/oauth/callback",
            "/verify-email",
        ])
}

/// Maps `SessionContext` + URL path into a CedarRequest.
///
/// Returns `None` when:
/// - No `SessionContext` in extensions (anonymous endpoint — login, signup, etc.)
/// - URL path doesn't match a ConnectRPC service/method shape
/// - Action belongs to a service whose resource lives in the request
///   body (Invitation actions targeting a specific token, etc.). For
///   those, the existing handler-side `require_*` keeps enforcing
///   until we add a `require_authorized(ctx, action, resource)` helper
///   in a later commit.
fn extract_cedar_request<B>(req: &http::Request<B>) -> Option<CedarRequest> {
    let session = req.extensions().get::<SessionContext>()?;
    let path = req.uri().path();
    let action = action_from_path(path)?;

    // Resource derivation by service.
    let resource = if path.contains("BillingService") {
        format!(r#"BillingAccount::"{}""#, session.billing.as_str())
            .parse::<EntityUid>()
            .ok()?
    } else if path.contains("OrgService") {
        let org = session.org.as_ref()?;
        format!(r#"Organization::"{}""#, org.as_str())
            .parse::<EntityUid>()
            .ok()?
    } else {
        return None;
    };

    let principal = format!(r#"User::"{}""#, session.user.as_str())
        .parse::<EntityUid>()
        .ok()?;

    let billing_uid = format!(r#"BillingAccount::"{}""#, session.billing.as_str())
        .parse::<EntityUid>()
        .ok()?;
    let role_str = match session.role {
        Role::Owner => "owner",
        Role::Member => "member",
    };
    let mut pairs = vec![
        (
            "billing".to_string(),
            RestrictedExpression::new_entity_uid(billing_uid),
        ),
        (
            "role".to_string(),
            RestrictedExpression::new_string(role_str.to_string()),
        ),
    ];
    if let Some(org_id) = &session.org {
        let org_uid = format!(r#"Organization::"{}""#, org_id.as_str())
            .parse::<EntityUid>()
            .ok()?;
        pairs.push((
            "org".to_string(),
            RestrictedExpression::new_entity_uid(org_uid),
        ));
    }
    let context = Context::from_pairs(pairs).ok()?;

    Some(CedarRequest {
        principal,
        action,
        resource,
        context,
    })
}
