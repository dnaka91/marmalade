#![forbid(unsafe_code)]
#![deny(rust_2018_idioms, clippy::all, clippy::pedantic)]
#![allow(clippy::manual_let_else, clippy::module_name_repetitions)]

use std::{
    env,
    net::{Ipv4Addr, SocketAddr},
};

use anyhow::Result;
use axum::{
    Router,
    routing::{get, post},
};
use tokio::net::TcpListener;
use tokio_shutdown::Shutdown;
use tower::{ServiceBuilder, util::AndThenLayer};
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tracing::{Level, info};
use tracing_subscriber::{filter::Targets, prelude::*};

use crate::{middleware::OnionLocationLayer, repositories::SettingsRepository};

mod assets;
mod cookies;
mod de;
mod dirs;
mod extract;
mod handlers;
mod middleware;
mod models;
mod redirect;
mod repositories;
mod response;
mod ser;
mod session;
mod templates;
mod validate;

const ADDRESS: Ipv4Addr = if cfg!(debug_assertions) {
    Ipv4Addr::LOCALHOST
} else {
    Ipv4Addr::UNSPECIFIED
};

#[tokio::main]
async fn main() -> Result<()> {
    SettingsRepository::init().await?;

    init_logging();

    let addr = SocketAddr::from((ADDRESS, 8080));
    let shutdown = Shutdown::new()?;

    let listener = TcpListener::bind(addr).await?;
    let server = axum::serve(
        listener,
        Router::new()
            .route("/{user}/{repo}/{service}", post(handlers::git::pack))
            .route("/{user}/{repo}/info/refs", get(handlers::git::info_refs))
            .route("/{user}/{repo}/tree/", get(handlers::repo::tree))
            .route("/{user}/{repo}/tree/{*path}", get(handlers::repo::tree))
            .route(
                "/{user}/{repo}/delete",
                get(handlers::repo::delete).post(handlers::repo::delete_post),
            )
            .route(
                "/{user}/{repo}/settings",
                get(handlers::repo::settings).post(handlers::repo::settings_post),
            )
            .route("/{user}/{repo}", get(handlers::repo::index))
            .route("/{user}/password", post(handlers::user::password_post))
            .route(
                "/{user}/settings",
                get(handlers::user::settings).post(handlers::user::settings_post),
            )
            .route("/{user}", get(handlers::user::index))
            .route(assets::WEBFONTS_ROUTE, get(handlers::assets::webfonts))
            .route(assets::MAIN_CSS_ROUTE, get(handlers::assets::main_css))
            .route(
                assets::FAVICON_SVG_ROUTE,
                get(handlers::assets::favicon_svg),
            )
            .route("/settings/dz", post(handlers::admin::settings_dz_post))
            .route("/settings/tor", post(handlers::admin::settings_tor_post))
            .route("/settings", get(handlers::admin::settings))
            .route("/users", get(handlers::user::list))
            .route(
                "/repo/create",
                get(handlers::repo::create).post(handlers::repo::create_post),
            )
            .route(
                "/register",
                get(handlers::auth::register).post(handlers::auth::register_post),
            )
            .route("/logout", post(handlers::auth::logout))
            .route(
                "/login",
                get(handlers::auth::login).post(handlers::auth::login_post),
            )
            .route("/", get(handlers::index))
            .fallback(handlers::handle_404)
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(CompressionLayer::new())
                    .layer(OnionLocationLayer::new())
                    .layer(AndThenLayer::new(middleware::security_headers))
                    .into_inner(),
            )
            .with_state(AppState {})
            .into_make_service(),
    )
    .with_graceful_shutdown(shutdown.handle());

    info!("Listening on http://{addr}");

    server.await?;

    Ok(())
}

#[derive(Clone)]
pub struct AppState {}

fn init_logging() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            Targets::new()
                .with_target(env!("CARGO_PKG_NAME"), Level::TRACE)
                .with_target("tower_http", Level::TRACE)
                .with_default(Level::INFO),
        )
        .init();
}
