use axum::{extract::Form, http::StatusCode, response::IntoResponse};
use serde::Deserialize;
use tracing::info;

use crate::{
    cookies::{Cookie, Cookies},
    extract::User,
    redirect,
    repositories::{SettingsRepository, UserRepository},
    response::{SetCookies, StatusTemplate},
    session::COOKIE_MESSAGE,
    templates,
};

pub async fn settings(
    User(user): User,
    mut cookies: Cookies,
) -> Result<impl IntoResponse, StatusTemplate> {
    info!(?user.username, "got admin settings request");

    let user_repo = UserRepository::for_user(&user.username);
    let settings_repo = SettingsRepository::new();

    if !user_repo.exists().await || !user_repo.load_info().await.unwrap().admin {
        return Err(StatusTemplate(StatusCode::NOT_FOUND));
    }

    let message = cookies
        .get(COOKIE_MESSAGE)
        .and_then(|cookie| cookie.value().parse().ok());

    if message.is_some() {
        cookies.remove(COOKIE_MESSAGE);
    }

    Ok(SetCookies::new(
        templates::admin::Settings {
            auth_user: Some(user),
            message,
            onion: settings_repo.get_tor_onion().await.unwrap_or_default(),
        },
        cookies,
    ))
}

#[derive(Deserialize)]
#[serde(tag = "kind")]
pub enum DangerZone {
    ResetKey,
}

pub async fn settings_dz_post(
    User(user): User,
    Form(danger_zone): Form<DangerZone>,
    mut cookies: Cookies,
) -> Result<impl IntoResponse, StatusTemplate> {
    info!(?user.username, "got admin settings request (danger zone)");

    let user_repo = UserRepository::for_user(&user.username);
    let settings_repo = SettingsRepository::new();

    if !user_repo.exists().await || !user_repo.load_info().await.unwrap().admin {
        return Err(StatusTemplate(StatusCode::NOT_FOUND));
    }

    match danger_zone {
        DangerZone::ResetKey => {
            settings_repo.reset_key().await.unwrap();

            for user in user_repo.list_user_names(&user.username).await.unwrap() {
                UserRepository::for_user(&user)
                    .clear_tokens()
                    .await
                    .unwrap();
            }
        }
    }

    cookies.add(Cookie::new(
        COOKIE_MESSAGE,
        templates::admin::ServerSettingsMessage::Success.as_ref(),
    ));

    Ok(SetCookies::new(redirect::to_admin_settings(), cookies))
}

#[derive(Deserialize)]
pub struct TorSettings {
    onion: String,
}

pub async fn settings_tor_post(
    User(user): User,
    Form(settings): Form<TorSettings>,
    mut cookies: Cookies,
) -> Result<impl IntoResponse, StatusTemplate> {
    info!(?user.username, "got admin settings request (tor)");

    let user_repo = UserRepository::for_user(&user.username);
    let settings_repo = SettingsRepository::new();

    if !user_repo.exists().await || !user_repo.load_info().await.unwrap().admin {
        return Err(StatusTemplate(StatusCode::NOT_FOUND));
    }

    settings_repo.set_tor_onion(settings.onion).await.unwrap();

    cookies.add(Cookie::new(
        COOKIE_MESSAGE,
        templates::admin::ServerSettingsMessage::Success.as_ref(),
    ));

    Ok(SetCookies::new(redirect::to_admin_settings(), cookies))
}
