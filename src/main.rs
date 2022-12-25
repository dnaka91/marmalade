#![forbid(unsafe_code)]
#![deny(rust_2018_idioms, clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![recursion_limit = "256"]

use std::{
    env,
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use axum::{
    extract::FromRef,
    routing::{get, post},
    Router, Server,
};
use tokio::sync::Mutex;
use tokio_shutdown::Shutdown;
use tower::{util::AndThenLayer, ServiceBuilder};
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tracing::{info, Level, Subscriber};
use tracing_archer::QuiverLayer;
use tracing_subscriber::{filter::Targets, prelude::*, registry::LookupSpan, reload, Registry};

use crate::{middleware::OnionLocationLayer, models::Archer, repositories::SettingsRepository};

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

    let toggle = init_logging(SettingsRepository::new().get_tracing_archer().await).await?;

    let addr = SocketAddr::from((ADDRESS, 8080));
    let shutdown = Shutdown::new()?;

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
                .route(assets::FAVICON_SVG_ROUTE, get(handlers::assets::favicon_svg))
                .route("/settings/dz", post(handlers::admin::settings_dz_post))
                .route("/settings/tor", post(handlers::admin::settings_tor_post))
                .route(
                    "/settings/tracing",
                    post(handlers::admin::settings_tracing_post),
                )
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
                .with_state(AppState {
                    toggle: Arc::new(toggle),
                })
                .into_make_service(),
        )
        .with_graceful_shutdown(shutdown.handle());

    info!("Listening on http://{addr}");

    server.await?;

    Ok(())
}

#[derive(Clone)]
pub struct AppState {
    toggle: Arc<TracingToggle>,
}

pub struct TracingToggle {
    reload: reload::Handle<Option<QuiverLayer<Registry>>, Registry>,
    handle: Mutex<Option<tracing_archer::Handle>>,
}

impl FromRef<AppState> for Arc<TracingToggle> {
    fn from_ref(input: &AppState) -> Self {
        Arc::clone(&input.toggle)
    }
}

#[allow(clippy::missing_errors_doc)]
impl TracingToggle {
    pub async fn enable(&self, archer: Archer) -> Result<()> {
        let (layer, handle) = init_tracing(archer).await?;
        self.reload.reload(Some(layer))?;
        if let Some(handle) = self.handle.lock().await.replace(handle) {
            handle.shutdown(Duration::from_secs(10)).await;
        }

        Ok(())
    }

    pub async fn disable(&self) -> Result<()> {
        self.reload.reload(None)?;
        if let Some(handle) = self.handle.lock().await.take() {
            handle.shutdown(Duration::from_secs(10)).await;
        }

        Ok(())
    }
}

async fn init_logging(archer: Option<Archer>) -> Result<TracingToggle> {
    let (tracing, handle) = match archer {
        Some(settings) => {
            let (layer, handle) = init_tracing(settings).await?;
            (Some(layer), Some(handle))
        }
        None => (None, None),
    };
    let (tracing, reload) = reload::Layer::new(tracing);

    tracing_subscriber::registry()
        .with(tracing)
        .with(tracing_subscriber::fmt::layer())
        .with(
            Targets::new()
                .with_target(env!("CARGO_PKG_NAME"), Level::TRACE)
                .with_target("tower_http", Level::TRACE)
                .with_default(Level::INFO),
        )
        .init();

    Ok(TracingToggle {
        reload,
        handle: Mutex::new(handle),
    })
}

async fn init_tracing<S>(settings: Archer) -> Result<(QuiverLayer<S>, tracing_archer::Handle)>
where
    for<'span> S: Subscriber + LookupSpan<'span>,
{
    tracing_archer::builder()
        .with_server_addr(settings.address)
        .with_server_cert(settings.certificate)
        .with_resource(env!("CARGO_CRATE_NAME"), env!("CARGO_PKG_VERSION"))
        .build()
        .await
        .map_err(Into::into)
}
