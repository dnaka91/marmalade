use axum::{
    extract::Form,
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
    session::{COOKIE_ERROR, COOKIE_SESSION, COOKIE_USERNAME},
    templates,
};

pub async fn login(mut cookies: Cookies) -> impl IntoResponse {
    let message = cookies
        .get(COOKIE_ERROR)
        .map(|cookie| cookie.value() == "login")
        .and_then(|error| error.then(|| "Wrong login credentials"));

    if message.is_some() {
        cookies.remove(COOKIE_ERROR);
    }

    SetCookies::new(HtmlTemplate(templates::Login { message }), cookies)
}

#[derive(Deserialize)]
pub struct Login {
    username: String,
    password: String,
}

pub async fn login_post(Form(login): Form<Login>, mut cookies: Cookies) -> impl IntoResponse {
    info!(?login.username, ?login.password, "got login request");

    let user_repo = UserRepository::for_user(&login.username);

    if !user_repo.exists().await || !user_repo.is_valid_password(&login.password).await.unwrap() {
        cookies.add(Cookie::new(COOKIE_ERROR, "login"));

        return SetCookies::new(Redirect::to("/login".parse().unwrap()), cookies);
    }

    let new_token = Uuid::new_v4();
    user_repo.add_token(new_token).await.unwrap();

    cookies.add(Cookie::new(COOKIE_SESSION, new_token.to_string()));
    cookies.add(Cookie::new(COOKIE_USERNAME, login.username));

    SetCookies::new(Redirect::to("/show".parse().unwrap()), cookies)
}

pub async fn logout(user: User, mut cookies: Cookies) -> impl IntoResponse {
    info!(?user.username, "got logout request");

    let user_repo = UserRepository::for_user(&user.username);
    user_repo.remove_token(user.token).await.unwrap();

    cookies.remove(COOKIE_SESSION);
    cookies.remove(COOKIE_USERNAME);

    SetCookies::new(Redirect::to("/".parse().unwrap()), cookies)
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
    let message = cookies
        .get(COOKIE_ERROR)
        .map(|cookie| cookie.value() == "register_exists")
        .and_then(|error| error.then(|| "The username is not available"));

    if message.is_some() {
        cookies.remove(COOKIE_ERROR);
    }

    SetCookies::new(HtmlTemplate(templates::Register { message }), cookies)
}

pub async fn register_post(Form(login): Form<Login>, mut cookies: Cookies) -> impl IntoResponse {
    info!(?login.username, ?login.password, "got register request");

    let user_repo = UserRepository::for_user(&login.username);
    let created = user_repo.create_user(&login.password, false).await.unwrap();

    if created {
        let new_token = Uuid::new_v4();
        user_repo.add_token(new_token).await.unwrap();

        cookies.add(Cookie::new(COOKIE_SESSION, new_token.to_string()));
        cookies.add(Cookie::new(COOKIE_USERNAME, login.username));

        SetCookies::new(Redirect::to("/".parse().unwrap()), cookies)
    } else {
        cookies.add(Cookie::new(COOKIE_ERROR, "register_exists"));

        SetCookies::new(Redirect::to("/register".parse().unwrap()), cookies)
    }
}
