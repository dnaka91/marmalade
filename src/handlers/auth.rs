use axum::{extract::Form, response::IntoResponse};
use serde::Deserialize;
use tracing::info;
use uuid::Uuid;

use crate::{
    cookies::{Cookie, Cookies},
    extract::BasicUser,
    redirect,
    repositories::{CreateUser, UserRepository},
    response::SetCookies,
    session::{COOKIE_ERROR, COOKIE_SESSION, COOKIE_USERNAME},
    templates, validate,
};

pub async fn login(mut cookies: Cookies) -> impl IntoResponse {
    info!("got auth login request");

    let error = cookies
        .get(COOKIE_ERROR)
        .and_then(|cookie| cookie.value().parse().ok());

    if error.is_some() {
        cookies.remove(COOKIE_ERROR);
    }

    SetCookies::new(templates::Login { error }, cookies)
}

#[derive(Deserialize)]
pub struct Login {
    username: String,
    password: String,
}

pub async fn login_post(mut cookies: Cookies, Form(login): Form<Login>) -> impl IntoResponse {
    info!(?login.username, "got auth login request");

    if login.username.is_empty() || login.password.is_empty() {
        cookies.add(Cookie::new(
            COOKIE_ERROR,
            templates::LoginError::Empty.as_ref(),
        ));
        return SetCookies::new(redirect::to_login(), cookies);
    }

    let user_repo = UserRepository::for_user(&login.username);

    if !user_repo.exists().await || !user_repo.is_valid_password(&login.password).await.unwrap() {
        cookies.add(Cookie::new(
            COOKIE_ERROR,
            templates::LoginError::UnknownUser.as_ref(),
        ));
        return SetCookies::new(redirect::to_login(), cookies);
    }

    let new_token = Uuid::new_v4();
    user_repo.add_token(new_token).await.unwrap();

    cookies.add(Cookie::new(COOKIE_SESSION, new_token.to_string()));
    cookies.add(Cookie::new(COOKIE_USERNAME, login.username));

    SetCookies::new(redirect::to_root(), cookies)
}

pub async fn logout(user: BasicUser, mut cookies: Cookies) -> impl IntoResponse {
    info!(?user.username, "got auth logout request");

    let user_repo = UserRepository::for_user(&user.username);
    user_repo.remove_token(user.token).await.unwrap();

    cookies.remove(COOKIE_SESSION);
    cookies.remove(COOKIE_USERNAME);

    SetCookies::new(redirect::to_root(), cookies)
}

pub async fn register(mut cookies: Cookies) -> impl IntoResponse {
    info!("got auth register request");

    let error = cookies
        .get(COOKIE_ERROR)
        .and_then(|cookie| cookie.value().parse().ok());

    if error.is_some() {
        cookies.remove(COOKIE_ERROR);
    }

    SetCookies::new(templates::Register { error }, cookies)
}

#[derive(Deserialize)]
pub struct Register {
    username: String,
    password: String,
    #[serde(default, deserialize_with = "crate::de::form_bool")]
    private: bool,
}

pub async fn register_post(mut cookies: Cookies, Form(login): Form<Register>) -> impl IntoResponse {
    info!(?login.username, "got auth register request");

    if !validate::username(&login.username) {
        cookies.add(Cookie::new(
            COOKIE_ERROR,
            templates::RegisterError::InvalidUsername.as_ref(),
        ));
        return SetCookies::new(redirect::to_register(), cookies);
    }

    if !validate::password(&login.password) {
        cookies.add(Cookie::new(
            COOKIE_ERROR,
            templates::RegisterError::InvalidPassword.as_ref(),
        ));
        return SetCookies::new(redirect::to_register(), cookies);
    }

    let user_repo = UserRepository::for_user(&login.username);
    let created = user_repo
        .create_user(CreateUser {
            password: &login.password,
            description: None,
            private: login.private,
            admin: false,
        })
        .await
        .unwrap();

    if created {
        let new_token = Uuid::new_v4();
        user_repo.add_token(new_token).await.unwrap();

        cookies.add(Cookie::new(COOKIE_SESSION, new_token.to_string()));
        cookies.add(Cookie::new(COOKIE_USERNAME, login.username));

        SetCookies::new(redirect::to_root(), cookies)
    } else {
        cookies.add(Cookie::new(
            COOKIE_ERROR,
            templates::RegisterError::UsernameTaken.as_ref(),
        ));
        SetCookies::new(redirect::to_register(), cookies)
    }
}
