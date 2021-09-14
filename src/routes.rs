use axum::{
    handler::{get, post, Handler},
    routing::BoxRoute,
    AddExtensionLayer, Router,
};
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};

use crate::{handlers, settings::GlobalSettings};

pub fn build(settings: GlobalSettings) -> Router<BoxRoute> {
    Router::new()
        .route("/favicon-16x16.png", get(handlers::favicon_16))
        .route("/favicon-32x32.png", get(handlers::favicon_32))
        .nest(
            "/:user/:repo",
            Router::new()
                .route("/:service", post(handlers::git::pack))
                .route("/info/refs", get(handlers::git::info_refs)),
        )
        .route("/show", get(handlers::auth::show))
        .route(
            "/register",
            get(handlers::auth::register).post(handlers::auth::register_post),
        )
        .route("/logout", post(handlers::auth::logout))
        .route(
            "/login",
            get(handlers::auth::login).post(handlers::auth::login_post),
        )
        .route("/", get(handlers::hello))
        .or(handlers::handle_404.into_service())
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(AddExtensionLayer::new(settings))
                .into_inner(),
        )
        .check_infallible()
        .boxed()
}
