//! Optional macaroon auth middleware.
//!
//! Inspects the `Authorization: Bearer <token>` header on each incoming
//! request, verifies the macaroon, and inserts a [`SessionContext`] into
//! request extensions on success. Failures are silent — handlers that
//! require auth call `require_session(ctx)?` and surface
//! `Code::Unauthenticated` themselves.
//!
//! Why optional? Several RPCs (signup, login, password reset, get
//! invitation by token) don't want a session, and a strict layer would
//! force an awkward "is_public" allowlist. Treating the layer as a
//! decoder keeps the handler responsible for its own access policy.

use std::sync::Arc;
use std::task::{Context, Poll};

use connectrpc::ConnectError;
use http::header::AUTHORIZATION;
use tower::{Layer, Service};

use crate::auth::{Keyring, SessionContext, verify_session};
use crate::time::SharedClock;

#[derive(Clone)]
pub struct AuthLayer {
    keyring: Arc<Keyring>,
    clock: SharedClock,
}

impl AuthLayer {
    pub fn new(keyring: Arc<Keyring>, clock: SharedClock) -> Self {
        Self { keyring, clock }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;
    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            inner,
            keyring: Arc::clone(&self.keyring),
            clock: Arc::clone(&self.clock),
        }
    }
}

#[derive(Clone)]
pub struct AuthService<S> {
    inner: S,
    keyring: Arc<Keyring>,
    clock: SharedClock,
}

impl<S, B> Service<http::Request<B>> for AuthService<S>
where
    S: Service<http::Request<B>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: http::Request<B>) -> Self::Future {
        if let Some(token) = req
            .headers()
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(extract_bearer)
        {
            let now_unix = self.clock.now_unix_seconds();
            if let Ok(session) = verify_session(&self.keyring, now_unix, token) {
                req.extensions_mut().insert(session);
            }
        }
        self.inner.call(req)
    }
}

fn extract_bearer(s: &str) -> Option<&str> {
    let s = s.trim();
    let prefix = "Bearer ";
    if s.len() > prefix.len() && s[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(s[prefix.len()..].trim())
    } else {
        None
    }
}

/// Convenience for handlers: pluck the verified `SessionContext` from
/// `RequestContext.extensions` or fail with `Code::Unauthenticated`.
pub fn require_session(ctx: &connectrpc::RequestContext) -> Result<SessionContext, ConnectError> {
    ctx.extensions()
        .get::<SessionContext>()
        .cloned()
        .ok_or_else(|| ConnectError::unauthenticated("session token required"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_bearer_token_case_insensitively() {
        assert_eq!(extract_bearer("Bearer abc"), Some("abc"));
        assert_eq!(extract_bearer("bearer abc"), Some("abc"));
        assert_eq!(extract_bearer("BEARER  abc "), Some("abc"));
        assert!(extract_bearer("Basic abc").is_none());
        assert!(extract_bearer("").is_none());
    }
}
