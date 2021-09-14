#![allow(clippy::unused_async)]

use axum::{http::StatusCode, response::IntoResponse};

use crate::{
    extract::User,
    response::{HtmlTemplate, StatusTemplate},
    templates,
};

pub mod auth;
pub mod git;

pub async fn hello(user: Option<User>) -> impl IntoResponse {
    tracing::info!(?user);

    HtmlTemplate(templates::Index {
        logged_in: user.is_some(),
    })
}

pub async fn favicon_32() -> impl IntoResponse {
    include_bytes!("../../assets/favicon-32x32.png").as_ref()
}

pub async fn favicon_16() -> impl IntoResponse {
    include_bytes!("../../assets/favicon-16x16.png").as_ref()
}

pub async fn handle_404() -> impl IntoResponse {
    StatusTemplate('🤷', StatusCode::NOT_FOUND)
}
