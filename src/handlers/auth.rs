use axum::{extract::Form, response::IntoResponse};
use serde::Deserialize;
use tracing::info;
use uuid::Uuid;

use crate::{
    cookies::{Cookie, Cookies},
    extract::User,
    redirect,
    repositories::UserRepository,
    response::{HtmlTemplate, SetCookies},
    session::{COOKIE_ERROR, COOKIE_SESSION, COOKIE_USERNAME},
    templates, validate,
};

pub async fn login(mut cookies: Cookies) -> impl IntoResponse {
    let error = cookies
        .get(COOKIE_ERROR)
        .and_then(|cookie| cookie.value().parse().ok());

    if error.is_some() {
        cookies.remove(COOKIE_ERROR);
    }

    SetCookies::new(HtmlTemplate(templates::Login { error }), cookies)
}

#[derive(Deserialize)]
pub struct Login {
    username: String,
    password: String,
}

pub async fn login_post(Form(login): Form<Login>, mut cookies: Cookies) -> impl IntoResponse {
    info!(?login.username, "got login request");

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

pub async fn logout(user: User, mut cookies: Cookies) -> impl IntoResponse {
    info!(?user.username, "got logout request");

    let user_repo = UserRepository::for_user(&user.username);
    user_repo.remove_token(user.token).await.unwrap();

    cookies.remove(COOKIE_SESSION);
    cookies.remove(COOKIE_USERNAME);

    SetCookies::new(redirect::to_root(), cookies)
}

pub async fn show(user: User) -> impl IntoResponse {
    let user_repo = UserRepository::for_user(&user.username);

    let username = user_repo
        .is_valid_token(user.token)
        .await
        .unwrap()
        .then(|| user.username);

    HtmlTemplate(templates::Show { username })
}

pub async fn register(mut cookies: Cookies) -> impl IntoResponse {
    let error = cookies
        .get(COOKIE_ERROR)
        .and_then(|cookie| cookie.value().parse().ok());

    if error.is_some() {
        cookies.remove(COOKIE_ERROR);
    }

    SetCookies::new(HtmlTemplate(templates::Register { error }), cookies)
}

#[derive(Deserialize)]
pub struct Register {
    username: String,
    password: String,
    #[serde(default, deserialize_with = "crate::de::form_bool")]
    private: bool,
}

pub async fn register_post(Form(login): Form<Register>, mut cookies: Cookies) -> impl IntoResponse {
    info!(?login.username, "got register request");

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
        .create_user(&login.password, login.private, false)
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
