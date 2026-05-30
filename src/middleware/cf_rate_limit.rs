//! CF Rate Limiting binding wired into this worker.
//!
//! Implements `connectrpc_cf_rate_limit::RateLimiter` for `worker::RateLimiter`
//! (the binding type, not to be confused with the crate trait of the same
//! short name — qualified imports throughout). Consumer-side wiring per the
//! ../../crates/connectrpc-cf-rate-limit/README.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use connectrpc_cf_rate_limit::{
    IpKeyExtractor, RateLimitKeyExtractor, RateLimitLayer, RateLimitOutcome, RateLimiter,
};
use futures::future::{self, Either};
use worker::Delay;

/// Adapter from `worker::RateLimiter` (binding) → the crate's
/// `RateLimiter` trait. `Arc<worker::RateLimiter>` because the binding
/// is `!Clone` and we need it shared across the (cloned) tower service
/// instances.
pub struct CfRateLimiter(pub Arc<worker::RateLimiter>);

/// How long we wait for the CF Rate Limiting binding to answer before
/// giving up and fail-opening.
///
/// **Why this exists**: `wrangler dev` reports the binding as "Unsafe
/// Metadata / remote" and the `limit()` JS promise never resolves
/// locally (the binding handshakes but no Cloudflare backend responds).
/// Without a timeout the request hangs for ~15 seconds before
/// `wrangler dev` gives up. 500ms is well above real-world binding
/// latency (~5ms) but tight enough that a stuck dev session moves on
/// fast. Production should never hit this branch.
const BINDING_TIMEOUT_MS: u64 = 500;


#[async_trait]
impl RateLimiter for CfRateLimiter {
    async fn check(&self, key: String) -> RateLimitOutcome {
        // `worker::Delay` holds a JS closure → `!Send`. The crate's
        // `RateLimiter` trait requires Send futures (so tower::Service
        // bounds are happy). `worker::send::SendFuture::new` wraps a
        // !Send future and asserts Send — sound on CF Workers because
        // the runtime is single-threaded by construction.
        let inner = self.0.clone();
        worker::send::SendFuture::new(async move {
            let call = Box::pin(inner.limit(key));
            let timeout = Box::pin(Delay::from(Duration::from_millis(BINDING_TIMEOUT_MS)));
            match future::select(call, timeout).await {
                Either::Left((Ok(o), _)) if o.success => RateLimitOutcome::Allowed,
                Either::Left((Ok(_), _)) => RateLimitOutcome::Exceeded,
                Either::Left((Err(e), _)) => RateLimitOutcome::Error(e.to_string()),
                Either::Right(_) => RateLimitOutcome::Error(format!(
                    "binding timeout after {BINDING_TIMEOUT_MS}ms"
                )),
            }
        })
        .await
    }
}

/// Build the rate-limit layer in **observe** mode for now — log every
/// would-have-throttled event so we can verify the per-IP key derivation
/// is sane before flipping to enforce.
///
/// `B: 'static` for the body — same constraint as the other layers.
/// Returned layer is fully generic over the inner service via the
/// `impl RateLimitKeyExtractor` return position.
pub fn observe_layer<B: 'static>(
    binding: worker::RateLimiter,
) -> RateLimitLayer<CfRateLimiter, impl RateLimitKeyExtractor<B> + Clone> {
    RateLimitLayer::observe(CfRateLimiter(Arc::new(binding)), IpKeyExtractor::new())
        .skip_paths([
            // Public endpoints that AuthLayer also doesn't gate.
            // No rate-limit on health checks (uptime probes hammer them).
            "/healthz",
            // OAuth bounce — already short and bursty by design.
            "/oauth/callback",
            "/verify-email",
        ])
}
