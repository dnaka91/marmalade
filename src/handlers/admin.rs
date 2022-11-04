use std::sync::Arc;

use axum::{extract::Form, http::StatusCode, response::IntoResponse, Extension};
use serde::Deserialize;
use tracing::{info, instrument};

use crate::{
    cookies::{Cookie, Cookies},
    extract::User,
    models::Quiver,
    redirect,
    repositories::{SettingsRepository, UserRepository},
    response::{SetCookies, StatusTemplate},
    session::COOKIE_MESSAGE,
    templates, TracingToggle,
};

#[instrument(skip_all, fields(?user.username))]
pub async fn settings(
    User(user): User,
    mut cookies: Cookies,
) -> Result<impl IntoResponse, StatusTemplate> {
    info!("got admin settings request");

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
            quiver: settings_repo.get_tracing_quiver().await,
        },
        cookies,
    ))
}

#[derive(Deserialize)]
#[serde(tag = "kind")]
pub enum DangerZone {
    ResetKey,
}

#[instrument(skip_all, fields(?user.username))]
pub async fn settings_dz_post(
    User(user): User,
    Form(danger_zone): Form<DangerZone>,
    mut cookies: Cookies,
) -> Result<impl IntoResponse, StatusTemplate> {
    info!("got admin settings request (danger zone)");

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

#[instrument(skip_all, fields(?user.username))]
pub async fn settings_tor_post(
    User(user): User,
    Form(settings): Form<TorSettings>,
    mut cookies: Cookies,
) -> Result<impl IntoResponse, StatusTemplate> {
    info!("got admin settings request (tor)");

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

#[derive(Deserialize)]
pub struct TracingSettings {
    address: String,
    certificate: String,
}

#[instrument(skip_all, fields(?user.username))]
pub async fn settings_tracing_post(
    User(user): User,
    Form(settings): Form<TracingSettings>,
    Extension(toggle): Extension<Arc<TracingToggle>>,
    mut cookies: Cookies,
) -> Result<impl IntoResponse, StatusTemplate> {
    info!("got admin settings request (tracing)");

    let user_repo = UserRepository::for_user(&user.username);
    let settings_repo = SettingsRepository::new();

    if !user_repo.exists().await || !user_repo.load_info().await.unwrap().admin {
        return Err(StatusTemplate(StatusCode::NOT_FOUND));
    }

    let quiver =
        (!settings.address.is_empty() && !settings.certificate.is_empty()).then_some(Quiver {
            address: settings.address,
            certificate: settings.certificate,
        });

    settings_repo
        .set_tracing_quiver(quiver.clone())
        .await
        .unwrap();

    let res = if let Some(quiver) = quiver {
        toggle.enable(quiver).await
    } else {
        toggle.disable().await
    };

    if let Err(e) = res {
        tracing::error!(error = ?e, "failed toggling OTLP settings");
        return Err(StatusTemplate(StatusCode::INTERNAL_SERVER_ERROR));
    }

    cookies.add(Cookie::new(
        COOKIE_MESSAGE,
        templates::admin::ServerSettingsMessage::Success.as_ref(),
    ));

    Ok(SetCookies::new(redirect::to_admin_settings(), cookies))
}
