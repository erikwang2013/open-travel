// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use super::claims::AuthClaims;
use http::{HeaderMap, Request, Response, StatusCode};

/// Build an error response without any fallible step: for a valid StatusCode
/// plus String body, `builder().body()` cannot fail, so construct it totally
/// instead of unwrapping in non-test code.
pub fn error_response(status: StatusCode, body: impl Into<String>) -> Response<axum::body::Body> {
    let mut resp = Response::new(axum::body::Body::from(body.into()));
    *resp.status_mut() = status;
    resp
}

pub fn claims_from_request<B>(req: &Request<B>) -> Option<&AuthClaims> {
    req.extensions().get::<AuthClaims>()
}

pub fn extract_bearer(headers: &HeaderMap, header_name: &str) -> Option<String> {
    let value = headers.get(header_name)?.to_str().ok()?;
    value.strip_prefix("Bearer ").map(|t| t.trim().to_string())
}

pub fn extract_header(headers: &HeaderMap, name: &str) -> Option<String> {
    headers.get(name)?.to_str().ok().map(|v| v.to_string())
}

pub fn extract_query_param(query: Option<&str>, param: &str) -> Option<String> {
    let query = query?;
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next()?;
        if key == param {
            return parts.next().map(|v| v.to_string());
        }
    }
    None
}
