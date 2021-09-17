use axum::{
    extract::{Form, Path},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use tracing::info;

use crate::{
    cookies::{Cookie, Cookies},
    extract::User,
    repositories::{RepoRepository, UserRepository},
    response::{HtmlTemplate, SetCookies, StatusTemplate},
    session::COOKIE_ERROR,
    templates,
};

const CREATE_EMPTY_NAME: &str = "repo_create_empty_name";
const CREATE_EXISTS: &str = "repo_create_exists";

#[derive(Deserialize)]
pub struct BasePath {
    pub user: String,
    #[serde(deserialize_with = "crate::de::repo_name")]
    pub repo: String,
}

pub async fn index(
    _user: User,
    Path(path): Path<BasePath>,
) -> Result<impl IntoResponse, StatusTemplate> {
    info!(?path.user,?path.repo, "got repo index request");

    if RepoRepository::new(&path.user, &path.repo).exists().await {
        Ok(HtmlTemplate(templates::repo::Index { name: path.repo }))
    } else {
        Err(StatusTemplate(StatusCode::NOT_FOUND))
    }
}

pub async fn create(_user: User) -> impl IntoResponse {
    HtmlTemplate(templates::repo::Create { message: None })
}

#[derive(Deserialize)]
pub struct Create {
    name: String,
    #[serde(default, deserialize_with = "crate::de::form_bool")]
    private: bool,
}

pub async fn create_post(
    user: User,
    Form(create): Form<Create>,
    mut cookies: Cookies,
) -> impl IntoResponse {
    info!(?user.username, ?create.name, "got repo create request");

    if create.name.is_empty() {
        cookies.add(Cookie::new(COOKIE_ERROR, CREATE_EMPTY_NAME));
        return SetCookies::new(Redirect::to("/repo/create".parse().unwrap()), cookies);
    }

    let user_repo = UserRepository::for_user(&user.username);
    let created = user_repo
        .repo(&create.name)
        .create(create.private)
        .await
        .unwrap();

    if created {
        SetCookies::new(
            Redirect::to(
                format!("/{}/{}", user.username, create.name)
                    .parse()
                    .unwrap(),
            ),
            cookies,
        )
    } else {
        cookies.add(Cookie::new(COOKIE_ERROR, CREATE_EXISTS));
        SetCookies::new(Redirect::to("/repo/create".parse().unwrap()), cookies)
    }
}
