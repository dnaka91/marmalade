use axum::{
    async_trait,
    extract::{FromRequest, RequestParts, TypedHeader},
    http::{header::WWW_AUTHENTICATE, StatusCode},
};
use headers::{authorization::Basic, Authorization, HeaderMap, HeaderValue};
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

#[async_trait]
impl<B> FromRequest<B> for BasicUser
where
    B: Send,
{
    type Rejection = StatusTemplate;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
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

pub struct User(pub UserAccount);

#[async_trait]
impl<B> FromRequest<B> for User
where
    B: Send,
{
    type Rejection = StatusTemplate;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let user = BasicUser::from_request(req).await?;
        let repo = UserRepository::for_user(&user.username);

        repo.load_info()
            .await
            .map(Self)
            .map_err(|_| INTERNAL_SERVER_ERROR)
    }
}

pub struct BasicAuth {
    pub username: String,
}

#[async_trait]
impl<B> FromRequest<B> for BasicAuth
where
    B: Send,
{
    type Rejection = (HeaderMap, StatusTemplate);

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(auth)) =
            <TypedHeader<Authorization<Basic>>>::from_request(req)
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
