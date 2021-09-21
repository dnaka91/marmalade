use std::{borrow::Cow, convert::Infallible, fmt::Display};

use axum::{
    async_trait,
    extract::{Extension, FromRequest, RequestParts},
    http::{header::COOKIE, HeaderMap},
};

use crate::settings::GlobalSettings;

#[derive(Debug)]
pub struct Cookie(cookie::Cookie<'static>);

impl Cookie {
    pub fn new<'a>(name: impl Into<Cow<'a, str>>, value: impl Into<Cow<'a, str>>) -> Self {
        let mut cookie = cookie::Cookie::new(name, value);
        cookie.set_path("/");
        cookie.set_same_site(cookie::SameSite::Strict);
        cookie.set_http_only(true);
        cookie.set_secure(true);
        cookie.make_permanent();

        Self(cookie.into_owned())
    }

    pub fn value(&self) -> &str {
        self.0.value()
    }
}

pub struct Cookies {
    jar: cookie::CookieJar,
    key: cookie::Key,
}

impl Cookies {
    pub fn get(&self, name: &str) -> Option<Cookie> {
        self.jar.private(&self.key).get(name).map(Cookie)
    }

    pub fn add(&mut self, cookie: Cookie) {
        self.jar.private_mut(&self.key).add(cookie.0);
    }

    pub fn remove(&mut self, name: &str) {
        let mut cookie = cookie::Cookie::new(name, "");
        cookie.make_removal();

        self.jar.private_mut(&self.key).remove(cookie.into_owned());
    }

    pub fn delta(&self) -> impl Iterator<Item = impl Display + '_> + '_ {
        self.jar.delta()
    }
}

#[async_trait]
impl<B> FromRequest<B> for Cookies
where
    B: Send,
{
    type Rejection = Infallible;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let settings = <Extension<GlobalSettings>>::from_request(req)
            .await
            .expect("global settings extension missing");

        let jar = get_cookie_jar(req.headers());
        let key = cookie::Key::from(&settings.key);

        Ok(Self { jar, key })
    }
}

fn get_cookie_jar(map: Option<&HeaderMap>) -> cookie::CookieJar {
    let mut jar = cookie::CookieJar::new();

    if let Some(map) = map {
        for cookie in map.get_all(COOKIE) {
            let cookie = match cookie.to_str() {
                Ok(value) => value,
                Err(_) => continue,
            };

            for part in cookie.split(';').map(|c| c.trim()) {
                if let Ok(cookie) = part.parse() {
                    jar.add_original(cookie);
                }
            }
        }
    }

    jar
}
