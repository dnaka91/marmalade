use axum::{extract::Path, http::StatusCode, response::IntoResponse};
use serde::Deserialize;
use tracing::info;

use crate::{
    extract::User,
    repositories::UserRepository,
    response::{HtmlTemplate, StatusTemplate},
    templates,
};

#[derive(Deserialize)]
pub struct BasePath {
    #[serde(deserialize_with = "crate::de::percent")]
    pub user: String,
}

pub async fn index(
    user: User,
    Path(path): Path<BasePath>,
) -> Result<impl IntoResponse, StatusTemplate> {
    info!(?path.user, "got user index request");

    let user_repo = UserRepository::for_user(&path.user);

    if user_repo.exists().await && user_repo.visible(&user.username, &path.user).await.unwrap() {
        let repos = user_repo.list_repo_names(&user.username).await.unwrap();

        Ok(HtmlTemplate(templates::user::Index {
            auth_user: Some(user.username),
            user: path.user,
            repos,
        }))
    } else {
        Err(StatusTemplate(StatusCode::NOT_FOUND))
    }
}

pub async fn list(user: User) -> impl IntoResponse {
    info!("got user list request");

    let users = UserRepository::for_user(&user.username)
        .list_user_names(&user.username)
        .await
        .unwrap();

    HtmlTemplate(templates::user::List {
        auth_user: Some(user.username),
        users,
    })
}
