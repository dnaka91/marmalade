use axum::{
    async_trait,
    extract::{FromRequest, RequestParts},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    cookies::Cookies,
    response::StatusTemplate,
    session::{COOKIE_SESSION, COOKIE_USERNAME},
};

#[derive(Debug)]
pub struct User {
    pub username: String,
    pub token: Uuid,
}

impl User {
    fn from_cookies(cookies: &Cookies) -> Option<Self> {
        let username = cookies.get(COOKIE_USERNAME)?.value().to_owned();
        let token = cookies.get(COOKIE_SESSION)?.value().parse().ok()?;

        Some(Self { username, token })
    }
}

#[async_trait]
impl<B> FromRequest<B> for User
where
    B: Send,
{
    type Rejection = StatusTemplate;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        Cookies::from_request(req)
            .await
            .ok()
            .and_then(|cookies| Self::from_cookies(&cookies))
            .ok_or(StatusTemplate('🙅', StatusCode::FORBIDDEN))
    }
}
