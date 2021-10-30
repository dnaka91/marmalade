#![allow(clippy::unused_async)]

use std::{
    convert::Infallible,
    mem,
    sync::Arc,
    task::{Context, Poll},
};

use axum::{
    body::BoxBody,
    http::{
        header::{
            CONTENT_SECURITY_POLICY, REFERRER_POLICY, STRICT_TRANSPORT_SECURITY,
            X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS, X_XSS_PROTECTION,
        },
        HeaderValue, Request, Response,
    },
};
use futures_util::future::BoxFuture;
use tower::{Layer, Service};
use tracing::error;

pub async fn security_headers(mut res: Response<BoxBody>) -> Result<Response<BoxBody>, Infallible> {
    let headers = res.headers_mut();

    headers.append(
        CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'none'; \
            font-src 'self'; \
            img-src 'self'; \
            style-src 'self';",
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

#[derive(Clone)]
pub struct OnionLocation<S> {
    inner: S,
    onion: Arc<Option<String>>,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for OnionLocation<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let onion = Arc::clone(&self.onion);

        // best practice is to clone the inner service like this
        // see https://github.com/tower-rs/tower/issues/547 for details
        let clone = self.inner.clone();
        let mut inner = mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            if let Some(onion) = onion.as_deref() {
                let path = req.uri().path().to_owned();
                let mut resp = inner.call(req).await?;

                match format!("{}{}", onion, path).try_into() {
                    Ok(loc) => {
                        resp.headers_mut().append("Onion-Location", loc);
                    }
                    Err(why) => {
                        error!(?path, ?why, "failed constructing onion location");
                    }
                }

                Ok(resp)
            } else {
                inner.call(req).await
            }
        })
    }
}

pub struct OnionLocationLayer {
    onion: Arc<Option<String>>,
}

impl OnionLocationLayer {
    pub fn new(onion: Option<&str>) -> Self {
        Self {
            onion: Arc::new(onion.map(ToOwned::to_owned)),
        }
    }
}

impl<S> Layer<S> for OnionLocationLayer {
    type Service = OnionLocation<S>;

    fn layer(&self, inner: S) -> Self::Service {
        OnionLocation {
            inner,
            onion: Arc::clone(&self.onion),
        }
    }
}
