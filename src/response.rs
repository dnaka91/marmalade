use std::convert::{Infallible, TryInto};

use askama::Template;
use axum::{
    body::{Bytes, Full},
    http::{header::SET_COOKIE, Response, StatusCode},
    response::{self, IntoResponse},
};
use tracing::error;

use crate::cookies::Cookies;

pub struct HtmlTemplate<T>(pub T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    type Body = Full<Bytes>;
    type BodyError = Infallible;

    fn into_response(self) -> Response<Self::Body> {
        match self.0.render() {
            Ok(html) => response::Html(html).into_response(),
            Err(e) => {
                error!("failed rendering template: {:?}", e);
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Full::default())
                    .unwrap()
            }
        }
    }
}

pub struct SetCookies<T> {
    inner: T,
    cookies: Cookies,
}

impl<'a, T> SetCookies<T> {
    pub fn new(inner: T, cookies: Cookies) -> Self {
        Self { inner, cookies }
    }
}

impl<T> IntoResponse for SetCookies<T>
where
    T: IntoResponse,
{
    type Body = T::Body;
    type BodyError = T::BodyError;

    fn into_response(self) -> Response<Self::Body> {
        let mut res = self.inner.into_response();
        let headers = res.headers_mut();

        for cookie in self.cookies.delta() {
            headers.append(SET_COOKIE, cookie.to_string().try_into().unwrap());
        }

        res
    }
}
