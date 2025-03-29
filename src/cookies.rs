use std::{borrow::Cow, convert::Infallible, fmt::Display};

use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, header::COOKIE, request::Parts},
};

use crate::repositories::SettingsRepository;

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

    fn removal<'a>(name: impl Into<Cow<'a, str>>) -> Self {
        let mut cookie = cookie::Cookie::new(name, "");
        cookie.set_path("/");
        cookie.set_same_site(cookie::SameSite::Strict);
        cookie.set_http_only(true);
        cookie.set_secure(true);
        cookie.make_removal();

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
        self.jar
            .private_mut(&self.key)
            .remove(Cookie::removal(name).0);
    }

    pub fn delta(&self) -> impl Iterator<Item = impl Display + '_> + '_ {
        self.jar.delta()
    }
}

impl<S> FromRequestParts<S> for Cookies
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let settings = SettingsRepository::new();

        let jar = get_cookie_jar(&parts.headers);
        let key = cookie::Key::from(&settings.get_key().await);

        Ok(Self { jar, key })
    }
}

pub fn generate_key() -> [u8; 64] {
    let mut key = [0; 64];
    rand::fill(&mut key);
    key
}

fn get_cookie_jar(map: &HeaderMap) -> cookie::CookieJar {
    let mut jar = cookie::CookieJar::new();

    for cookie in map.get_all(COOKIE) {
        let cookie = match cookie.to_str() {
            Ok(value) => value,
            Err(_) => continue,
        };

        for part in cookie.split(';').map(str::trim) {
            if let Ok(cookie) = part.parse::<cookie::Cookie<'static>>() {
                jar.add_original(cookie);
            }
        }
    }

    jar
}
