use axum::{
    extract::Form,
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use tracing::info;
use uuid::Uuid;

use crate::{
    cookies::{Cookie, Cookies},
    extract::User,
    repositories::UserRepository,
    response::{HtmlTemplate, SetCookies},
    session::{COOKIE_SESSION, COOKIE_USERNAME},
    templates,
};

pub async fn login() -> impl IntoResponse {
    HtmlTemplate(templates::Login)
}

#[derive(Deserialize)]
pub struct Login {
    username: String,
    password: String,
}

pub async fn login_post(Form(login): Form<Login>, mut cookies: Cookies) -> impl IntoResponse {
    info!(?login.username, ?login.password, "got login request");

    let user_repo = UserRepository;
    let new_token = Uuid::new_v4();

    user_repo
        .add_token(&login.username, new_token)
        .await
        .unwrap();

    cookies.add(Cookie::new(COOKIE_SESSION, new_token.to_string()));
    cookies.add(Cookie::new(COOKIE_USERNAME, login.username));

    SetCookies::new(Redirect::to("/show".parse().unwrap()), cookies)
}

pub async fn logout(user: User, mut cookies: Cookies) -> impl IntoResponse {
    info!(?user.username, "got logout request");

    let user_repo = UserRepository;
    user_repo
        .remove_token(&user.username, user.token)
        .await
        .unwrap();

    cookies.remove(COOKIE_SESSION);
    cookies.remove(COOKIE_USERNAME);

    SetCookies::new(Redirect::to("/".parse().unwrap()), cookies)
}

pub async fn show(user: User) -> impl IntoResponse {
    let user_repo = UserRepository;

    let username = user_repo
        .is_valid_token(&user.username, user.token)
        .await
        .unwrap()
        .then(|| user.username);

    HtmlTemplate(templates::Show { username })
}

pub async fn register() -> impl IntoResponse {
    HtmlTemplate(templates::Register)
}

pub async fn register_post(Form(login): Form<Login>) -> impl IntoResponse {
    info!(?login.username, ?login.password, "got register request");

    let user_repo = UserRepository;
    let exists = user_repo
        .create_user(&login.username, &login.password, false)
        .await
        .unwrap();

    Redirect::to("/".parse().unwrap())
}
