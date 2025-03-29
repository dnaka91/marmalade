use axum::{
    extract::FromRequestParts,
    http::{StatusCode, header::WWW_AUTHENTICATE, request::Parts},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, HeaderMap, HeaderValue, authorization::Basic},
};
use uuid::Uuid;

use crate::{
    cookies::Cookies,
    models::UserAccount,
    repositories::UserRepository,
    response::StatusTemplate,
    session::{COOKIE_SESSION, COOKIE_USERNAME},
};

const FORBIDDEN: StatusTemplate = StatusTemplate(StatusCode::FORBIDDEN);
const INTERNAL_SERVER_ERROR: StatusTemplate = StatusTemplate(StatusCode::INTERNAL_SERVER_ERROR);
const UNAUTHORIZED: StatusTemplate = StatusTemplate(StatusCode::UNAUTHORIZED);

pub struct BasicUser {
    pub username: String,
    pub token: Uuid,
}

impl BasicUser {
    fn from_cookies(cookies: &Cookies) -> Option<Self> {
        let username = cookies.get(COOKIE_USERNAME)?.value().to_owned();
        let token = cookies.get(COOKIE_SESSION)?.value().parse().ok()?;

        Some(Self { username, token })
    }
}

impl<S> FromRequestParts<S> for BasicUser
where
    S: Send + Sync,
{
    type Rejection = StatusTemplate;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = Cookies::from_request_parts(parts, state)
            .await
            .ok()
            .and_then(|cookies| Self::from_cookies(&cookies))
            .ok_or(FORBIDDEN)?;

        let repo = UserRepository::for_user(&user.username);

        if repo.exists().await {
            repo.is_valid_token(user.token)
                .await
                .map_err(|_| INTERNAL_SERVER_ERROR)?
                .then_some(user)
                .ok_or(FORBIDDEN)
        } else {
            Err(FORBIDDEN)
        }
    }
}

pub struct User(pub UserAccount);

impl<S> FromRequestParts<S> for User
where
    S: Send + Sync,
{
    type Rejection = StatusTemplate;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = BasicUser::from_request_parts(parts, state).await?;
        let repo = UserRepository::for_user(&user.username);

        repo.load_info()
            .await
            .map(Self)
            .map_err(|_| INTERNAL_SERVER_ERROR)
    }
}

impl<S> axum::extract::OptionalFromRequestParts<S> for User
where
    S: Send + Sync,
{
    type Rejection = StatusTemplate;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        let Ok(user) = BasicUser::from_request_parts(parts, state).await else {
            return Ok(None);
        };
        let repo = UserRepository::for_user(&user.username);

        repo.load_info()
            .await
            .map(Self)
            .map(Some)
            .map_err(|_| INTERNAL_SERVER_ERROR)
    }
}

pub struct BasicAuth {
    pub username: String,
}

impl<S> FromRequestParts<S> for BasicAuth
where
    S: Send + Sync,
{
    type Rejection = (HeaderMap, StatusTemplate);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(auth)) =
            <TypedHeader<Authorization<Basic>>>::from_request_parts(parts, state)
                .await
                .map_err(|_| {
                    let mut headers = HeaderMap::with_capacity(1);
                    headers.insert(WWW_AUTHENTICATE, HeaderValue::from_static("Basic"));

                    (headers, UNAUTHORIZED)
                })?;

        let repo = UserRepository::for_user(auth.username());

        if repo.exists().await {
            repo.is_valid_password(auth.password())
                .await
                .map_err(|_| (HeaderMap::new(), INTERNAL_SERVER_ERROR))?
                .then(|| BasicAuth {
                    username: auth.username().to_owned(),
                })
                .ok_or((HeaderMap::new(), FORBIDDEN))
        } else {
            Err((HeaderMap::new(), FORBIDDEN))
        }
    }
}
