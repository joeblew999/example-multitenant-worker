//! Non-RPC HTTP routes — health, OAuth callback stub, and the
//! email-verification handoff. Short-circuited ahead of the RPC stack.

use bytes::Bytes;
use connectrpc::ConnectRpcBody;
use http::{Method, Response, StatusCode};
use http_body_util::Full;
use worker::HttpRequest;

pub fn try_handle(req: &HttpRequest) -> Option<Response<ConnectRpcBody>> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/healthz") => Some(healthz()),
        (&Method::GET, "/oauth/callback") => Some(oauth_callback(req.uri().query())),
        (&Method::GET, "/verify-email") => Some(verify_email_stub(req.uri().query())),
        _ => None,
    }
}

fn text(status: StatusCode, body: impl Into<Bytes>) -> Response<ConnectRpcBody> {
    Response::builder()
        .status(status)
        .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(ConnectRpcBody::Full(Full::new(body.into())))
        .expect("static response builder inputs are valid")
}

fn healthz() -> Response<ConnectRpcBody> {
    text(StatusCode::OK, "ok")
}

fn oauth_callback(query: Option<&str>) -> Response<ConnectRpcBody> {
    let (mut code, mut state) = (None, None);
    for (k, v) in parse_query(query.unwrap_or_default()) {
        match k.as_ref() {
            "code" => code = Some(v.into_owned()),
            "state" => state = Some(v.into_owned()),
            _ => {}
        }
    }
    let Some(code) = code else {
        return text(
            StatusCode::BAD_REQUEST,
            "missing `code` query parameter; pass through Auth.SsoComplete instead",
        );
    };
    let state = state.unwrap_or_default();
    text(
        StatusCode::OK,
        format!(
            "OAuth callback received: code={code}, state={state}.\n\
             For the demo this is a stub — call Auth.SsoComplete with the same `state`."
        ),
    )
}

/// Plumbed but bypassed: per `plan.md`, email verification is
/// auto-treated as completed at signup time. Flipping
/// `ENFORCE_EMAIL_VERIFICATION=true` and minting a real verify-email
/// macaroon would route through here.
fn verify_email_stub(_query: Option<&str>) -> Response<ConnectRpcBody> {
    text(
        StatusCode::OK,
        "email verification is bypassed in this build; flip ENFORCE_EMAIL_VERIFICATION to enable",
    )
}

fn parse_query(query: &str) -> url::form_urlencoded::Parse<'_> {
    url::form_urlencoded::parse(query.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use http_body_util::BodyExt;

    fn request(method: Method, uri: &str) -> HttpRequest {
        http::Request::builder()
            .method(method)
            .uri(uri)
            .body(worker::Body::empty())
            .unwrap()
    }

    fn read_body(resp: Response<ConnectRpcBody>) -> (StatusCode, String) {
        let status = resp.status();
        let bytes = block_on(resp.into_body().collect()).unwrap().to_bytes();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    }

    #[test]
    fn healthz_ok() {
        let resp = try_handle(&request(Method::GET, "/healthz")).unwrap();
        let (status, body) = read_body(resp);
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "ok");
    }

    #[test]
    fn rpc_paths_defer() {
        assert!(try_handle(&request(Method::POST, "/workers.auth.v1.AuthService/Login")).is_none());
    }

    #[test]
    fn oauth_callback_requires_code() {
        let resp = try_handle(&request(Method::GET, "/oauth/callback?state=x")).unwrap();
        let (status, _body) = read_body(resp);
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}
