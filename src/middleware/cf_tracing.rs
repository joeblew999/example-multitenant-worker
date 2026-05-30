//! CF-Workers tracing layer wired into this worker.
//!
//! Outermost layer on the stack — its span wraps every downstream
//! layer (Auth, Cedar) and the inner `ConnectRpcService`. Every event
//! logged anywhere inside an RPC handler picks up the active span's
//! CF fields automatically.
//!
//! Data source: workers-rs auto-inserts `worker::Cf` into the
//! `http::Request::extensions` (see worker-0.8.3/src/http/request.rs:41).
//! We pull it back out here, plus the `cf-ray` header, and stuff both
//! into `CfFields` for the tracing layer to span on.

use connectrpc_cf_tracing::{CfFields, CfFieldsExtractor, TracingLayer};
use http::Request;

/// Build the CF tracing layer for this worker. Wire it as the
/// **outermost** layer on the tower stack so every event logged
/// downstream gets the CF metadata.
pub fn tracing_layer<B: 'static>() -> TracingLayer<impl CfFieldsExtractor<B> + Clone> {
    TracingLayer::new(extract_cf_fields::<B>)
}

/// Convert workers-rs runtime extensions + headers into the typed
/// fields the tracing layer expects. Each field is best-effort — if
/// the runtime didn't populate something (mostly local-dev /
/// `wrangler dev`), the field is `None` and tracing renders the
/// missing one as blank rather than panicking.
fn extract_cf_fields<B>(req: &Request<B>) -> CfFields {
    let cf = req.extensions().get::<worker::Cf>();

    let procedure = Some(req.uri().path().to_string());
    let colo = cf.map(|c| c.colo());
    let country = cf.and_then(|c| c.country());
    let asn = cf.and_then(|c| c.asn());
    let tls_cipher = cf.map(|c| c.tls_cipher());
    let http_protocol = cf.map(|c| c.http_protocol());
    let ray = req
        .headers()
        .get("cf-ray")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    CfFields {
        procedure,
        colo,
        country,
        asn,
        tls_cipher,
        http_protocol,
        ray,
    }
}
