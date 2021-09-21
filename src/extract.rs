use axum::{
    async_trait,
    extract::{FromRequest, RequestParts},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    cookies::Cookies,
    repositories::UserRepository,
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
        const FORBIDDEN: StatusTemplate = StatusTemplate(StatusCode::FORBIDDEN);
        const INTERNAL_SERVER_ERROR: StatusTemplate =
            StatusTemplate(StatusCode::INTERNAL_SERVER_ERROR);

        let user = Cookies::from_request(req)
            .await
            .ok()
            .and_then(|cookies| Self::from_cookies(&cookies))
            .ok_or(FORBIDDEN)?;

        let repo = UserRepository::for_user(&user.username);

        if repo.exists().await {
            repo.is_valid_token(user.token)
                .await
                .map_err(|_| INTERNAL_SERVER_ERROR)?
                .then(|| user)
                .ok_or(FORBIDDEN)
        } else {
            Err(FORBIDDEN)
        }
    }
}
