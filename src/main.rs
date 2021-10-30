#![forbid(unsafe_code)]
#![deny(rust_2018_idioms, clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![recursion_limit = "256"]

use std::{env, net::SocketAddr};

use anyhow::Result;
use axum::{
    handler::Handler,
    routing::{get, post},
    AddExtensionLayer, Router, Server,
};
use tokio::signal;
use tower::{util::AndThenLayer, ServiceBuilder};
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tracing::{info, Level};
use tracing_subscriber::{filter::Targets, prelude::*};

use crate::middleware::OnionLocationLayer;

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
mod session;
mod settings;
mod templates;
mod validate;

#[cfg(debug_assertions)]
const ADDRESS: [u8; 4] = [127, 0, 0, 1];
#[cfg(not(debug_assertions))]
const ADDRESS: [u8; 4] = [0, 0, 0, 0];

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            Targets::new()
                .with_target(env!("CARGO_PKG_NAME"), Level::TRACE)
                .with_target("tower_http", Level::TRACE)
                .with_default(Level::INFO),
        )
        .init();

    let settings = crate::settings::load()?;
    let addr = SocketAddr::from((ADDRESS, 8080));

    let server = Server::try_bind(&addr)?
        .serve(
            Router::new()
                .route("/:user/:repo/:service", post(handlers::git::pack))
                .route("/:user/:repo/info/refs", get(handlers::git::info_refs))
                .route("/:user/:repo/tree/*path", get(handlers::repo::tree))
                .route(
                    "/:user/:repo/delete",
                    get(handlers::repo::delete).post(handlers::repo::delete_post),
                )
                .route(
                    "/:user/:repo/settings",
                    get(handlers::repo::settings).post(handlers::repo::settings_post),
                )
                .route("/:user/:repo", get(handlers::repo::index))
                .route("/:user/password", post(handlers::user::password_post))
                .route(
                    "/:user/settings",
                    get(handlers::user::settings).post(handlers::user::settings_post),
                )
                .route("/:user", get(handlers::user::index))
                .route(assets::WEBFONTS_ROUTE, get(handlers::assets::webfonts))
                .route(assets::MAIN_CSS_ROUTE, get(handlers::assets::main_css))
                .route("/favicon-16x16.png", get(handlers::assets::favicon_16))
                .route("/favicon-32x32.png", get(handlers::assets::favicon_32))
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
                .fallback(handlers::handle_404.into_service())
                .layer(
                    ServiceBuilder::new()
                        .layer(TraceLayer::new_for_http())
                        .layer(CompressionLayer::new())
                        .layer(OnionLocationLayer::new(
                            settings.tor.as_ref().map(|t| t.onion.as_str()),
                        ))
                        .layer(AddExtensionLayer::new(settings))
                        .layer(AndThenLayer::new(middleware::security_headers))
                        .into_inner(),
                )
                .into_make_service(),
        )
        .with_graceful_shutdown(shutdown());

    info!("Listening on {}", addr);

    server.await?;

    Ok(())
}

async fn shutdown() {
    signal::ctrl_c().await.ok();
    info!("Shutting down");
}
