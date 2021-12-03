use std::convert::TryInto;

use askama::Template;
use axum::{
    body::{self, BoxBody, Empty},
    http::{header::SET_COOKIE, Response, StatusCode},
    response::{self, IntoResponse},
};
use tracing::error;

use crate::{cookies::Cookies, templates};

pub struct HtmlTemplate<T>(pub T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response<BoxBody> {
        match self.0.render() {
            Ok(html) => response::Html(html).into_response(),
            Err(err) => {
                error!(?err, "failed rendering template");
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(body::boxed(Empty::new()))
                    .unwrap()
            }
        }
    }
}

pub struct StatusTemplate(pub StatusCode);

impl IntoResponse for StatusTemplate {
    fn into_response(self) -> Response<BoxBody> {
        let mut res = HtmlTemplate(templates::Error {
            code: self.0,
            message: None,
        })
        .into_response();

        if res.status() != StatusCode::INTERNAL_SERVER_ERROR {
            *res.status_mut() = self.0;
        }

        res
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
    fn into_response(self) -> Response<BoxBody> {
        let mut res = self.inner.into_response();
        let headers = res.headers_mut();

        for cookie in self.cookies.delta() {
            headers.append(SET_COOKIE, cookie.to_string().try_into().unwrap());
        }

        res
    }
}
