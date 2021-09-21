#![allow(clippy::unused_async)]

use std::convert::Infallible;

use axum::{
    body::BoxBody,
    http::{
        header::{
            CONTENT_SECURITY_POLICY, REFERRER_POLICY, STRICT_TRANSPORT_SECURITY,
            X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS, X_XSS_PROTECTION,
        },
        HeaderValue, Response,
    },
};

pub async fn security_headers(mut res: Response<BoxBody>) -> Result<Response<BoxBody>, Infallible> {
    let headers = res.headers_mut();

    headers.append(
        CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'none'; \
            font-src https://cdn.jsdelivr.net; \
            img-src 'self' https://cdn.jsdelivr.net; \
            style-src 'unsafe-inline' https://cdn.jsdelivr.net;",
        ),
    );
    headers.append(REFERRER_POLICY, "same-origin".parse().unwrap());
    headers.append(
        STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=63072000; includeSubDomains; preload"),
    );
    headers.append(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.append(X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.append(X_XSS_PROTECTION, HeaderValue::from_static("1; mode=block"));

    Ok(res)
}
