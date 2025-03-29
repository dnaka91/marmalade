use std::convert::TryInto;

use axum::{
    http::{StatusCode, header::SET_COOKIE},
    response::{IntoResponse, Response},
};

use crate::{cookies::Cookies, templates};

pub struct StatusTemplate(pub StatusCode);

impl IntoResponse for StatusTemplate {
    fn into_response(self) -> Response {
        let mut res = templates::Error {
            code: self.0,
            message: None,
        }
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

impl<T> SetCookies<T> {
    pub fn new(inner: T, cookies: Cookies) -> Self {
        Self { inner, cookies }
    }
}

impl<T> IntoResponse for SetCookies<T>
where
    T: IntoResponse,
{
    fn into_response(self) -> Response {
        let mut res = self.inner.into_response();
        let headers = res.headers_mut();

        for cookie in self.cookies.delta() {
            headers.append(SET_COOKIE, cookie.to_string().try_into().unwrap());
        }

        res
    }
}
