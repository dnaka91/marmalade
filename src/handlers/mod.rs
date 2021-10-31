#![allow(clippy::unused_async)]

use axum::{http::StatusCode, response::IntoResponse};
use tracing::info;

use crate::{
    extract::User,
    response::{HtmlTemplate, StatusTemplate},
    templates,
};

pub mod admin;
pub mod assets;
pub mod auth;
pub mod git;
pub mod repo;
pub mod user;

pub async fn index(user: Option<User>) -> impl IntoResponse {
    info!(authorized = user.is_some(), "got index request");

    HtmlTemplate(templates::Index {
        auth_user: user.map(|user| user.0),
    })
}

pub async fn handle_404() -> impl IntoResponse {
    StatusTemplate(StatusCode::NOT_FOUND)
}
