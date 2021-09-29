use axum::{
    extract::{Form, Path},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use tracing::info;

use crate::{
    cookies::{Cookie, Cookies},
    extract::User,
    redirect,
    repositories::UserRepository,
    response::{HtmlTemplate, SetCookies, StatusTemplate},
    session::COOKIE_MESSAGE,
    templates, validate,
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

pub async fn settings(
    user: User,
    Path(path): Path<BasePath>,
    mut cookies: Cookies,
) -> Result<impl IntoResponse, StatusTemplate> {
    info!(?path.user, "got user settings request");

    let user_repo = UserRepository::for_user(&path.user);

    if user.username != path.user || !user_repo.exists().await {
        return Err(StatusTemplate(StatusCode::NOT_FOUND));
    }

    let message = cookies
        .get(COOKIE_MESSAGE)
        .and_then(|cookie| cookie.value().parse().ok());

    if message.is_some() {
        cookies.remove(COOKIE_MESSAGE);
    }

    let settings = user_repo.load_info().await.unwrap();

    Ok(SetCookies::new(
        HtmlTemplate(templates::user::Settings {
            auth_user: Some(user.username),
            message,
            user: path.user,
            settings,
        }),
        cookies,
    ))
}

#[derive(Deserialize)]
pub struct Settings {
    description: String,
    #[serde(default, deserialize_with = "crate::de::form_bool")]
    private: bool,
}

pub async fn settings_post(
    user: User,
    Path(path): Path<BasePath>,
    Form(settings): Form<Settings>,
    mut cookies: Cookies,
) -> Result<impl IntoResponse, StatusTemplate> {
    info!(?path.user, "got user settings request");

    let user_repo = UserRepository::for_user(&path.user);

    if user.username != path.user || !user_repo.exists().await {
        return Err(StatusTemplate(StatusCode::NOT_FOUND));
    }

    let mut current = user_repo.load_info().await.unwrap();
    if current.description != settings.description || current.private != settings.private {
        current.description = settings.description;
        current.private = settings.private;
        user_repo.save_info(&current).await.unwrap();
    }

    cookies.add(Cookie::new(
        COOKIE_MESSAGE,
        templates::user::UserSettingsMessage::Success.as_ref(),
    ));

    Ok(SetCookies::new(
        redirect::to_user_settings(&path.user),
        cookies,
    ))
}

#[derive(Deserialize)]
pub struct NewPassword {
    password: String,
}

pub async fn password_post(
    user: User,
    Path(path): Path<BasePath>,
    Form(pw): Form<NewPassword>,
    mut cookies: Cookies,
) -> Result<impl IntoResponse, StatusTemplate> {
    info!(?path.user, "got user password request");

    let user_repo = UserRepository::for_user(&path.user);

    if user.username != path.user || !user_repo.exists().await {
        return Err(StatusTemplate(StatusCode::NOT_FOUND));
    }

    if !validate::password(&pw.password) {
        cookies.add(Cookie::new(
            COOKIE_MESSAGE,
            templates::user::UserSettingsMessage::InvalidPassword.as_ref(),
        ));
        return Ok(SetCookies::new(
            redirect::to_user_settings(&path.user),
            cookies,
        ));
    }

    user_repo.change_password(&pw.password).await.unwrap();

    cookies.add(Cookie::new(
        COOKIE_MESSAGE,
        templates::user::UserSettingsMessage::Success.as_ref(),
    ));

    Ok(SetCookies::new(redirect::to_root(), cookies))
}
