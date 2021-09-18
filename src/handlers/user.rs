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
    pub user: String,
}

pub async fn index(
    _user: User,
    Path(path): Path<BasePath>,
) -> Result<impl IntoResponse, StatusTemplate> {
    info!(?path.user, "got user index request");

    let user_repo = UserRepository::for_user(&path.user);

    if user_repo.exists().await {
        let repos = user_repo.list_repo_names().await.unwrap();

        Ok(HtmlTemplate(templates::user::Index {
            logged_in: true,
            user: path.user,
            repos,
        }))
    } else {
        Err(StatusTemplate(StatusCode::NOT_FOUND))
    }
}
