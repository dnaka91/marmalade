#![forbid(unsafe_code)]
#![deny(rust_2018_idioms, clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use std::{env, net::SocketAddr};

use anyhow::Result;
use axum::Server;
use tokio::signal;
use tracing::{info, Level};
use tracing_subscriber::{filter::Targets, prelude::*};

mod cookies;
mod dirs;
mod extract;
mod handlers;
mod models;
mod repositories;
mod response;
mod routes;
mod session;
mod settings;
mod templates;

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
        .serve(routes::build(settings).into_make_service())
        .with_graceful_shutdown(shutdown());

    info!("Listening on {}", addr);

    server.await?;

    Ok(())
}

async fn shutdown() {
    signal::ctrl_c().await.ok();
    info!("Shutting down");
}
