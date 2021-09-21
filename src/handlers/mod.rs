#![allow(clippy::unused_async)]

use axum::{http::StatusCode, response::IntoResponse};
use tracing::info;

use crate::{
    extract::User,
    response::{HtmlTemplate, StatusTemplate},
    templates,
};

pub mod auth;
pub mod git;
pub mod repo;
pub mod user;

pub async fn index(user: Option<User>) -> impl IntoResponse {
    info!(authorized = user.is_some(), "got index request");

    HtmlTemplate(templates::Index {
        auth_user: user.map(|user| user.username),
    })
}

pub async fn favicon_32() -> impl IntoResponse {
    include_bytes!("../../assets/favicon-32x32.png").as_ref()
}

pub async fn favicon_16() -> impl IntoResponse {
    include_bytes!("../../assets/favicon-16x16.png").as_ref()
}

pub async fn handle_404() -> impl IntoResponse {
    StatusTemplate(StatusCode::NOT_FOUND)
}
