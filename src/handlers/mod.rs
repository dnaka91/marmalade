#![allow(clippy::unused_async)]

use std::{collections::hash_map::DefaultHasher, hash::Hasher};

use axum::{extract::TypedHeader, http::StatusCode, response::IntoResponse};
use headers::{CacheControl, ContentType, ETag, HeaderMap, HeaderMapExt, IfNoneMatch};
use once_cell::sync::Lazy;
use syntect::{highlighting::ThemeSet, html::ClassStyle};
use tracing::info;

use crate::{
    extract::User,
    response::{HtmlTemplate, StatusTemplate},
    templates,
};

pub mod auth;
pub mod git;
pub mod repo;
pub mod user;

pub async fn index(user: Option<User>) -> impl IntoResponse {
    info!(authorized = user.is_some(), "got index request");

    HtmlTemplate(templates::Index {
        auth_user: user.map(|user| user.username),
    })
}

pub async fn favicon_32() -> impl IntoResponse {
    include_bytes!("../../assets/favicon-32x32.png").as_ref()
}

pub async fn favicon_16() -> impl IntoResponse {
    include_bytes!("../../assets/favicon-16x16.png").as_ref()
}

pub async fn highlight_css(
    if_none_match: Option<TypedHeader<IfNoneMatch>>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    static CSS: Lazy<String> = Lazy::new(|| {
        let theme_set = ThemeSet::load_defaults();
        syntect::html::css_for_theme_with_class_style(
            &theme_set.themes["base16-ocean.light"],
            ClassStyle::SpacedPrefixed {
                prefix: "highlight-",
            },
        )
    });
    static HASH: Lazy<ETag> = Lazy::new(|| {
        let mut hasher = DefaultHasher::new();
        hasher.write(CSS.as_bytes());

        format!("\"W/{:016x}\"", hasher.finish()).parse().unwrap()
    });

    if if_none_match.map_or(true, |v| v.precondition_passes(&*HASH)) {
        let mut headers = HeaderMap::with_capacity(3);
        headers.typed_insert(ContentType::from(mime::TEXT_CSS));
        headers.typed_insert(CacheControl::new().with_public());
        headers.typed_insert(HASH.clone());

        Ok((headers, CSS.as_str()))
    } else {
        let mut headers = HeaderMap::with_capacity(1);
        headers.typed_insert(CacheControl::new().with_public());
        headers.typed_insert(HASH.clone());

        Err((headers, StatusCode::NOT_MODIFIED))
    }
}

pub async fn handle_404() -> impl IntoResponse {
    StatusTemplate(StatusCode::NOT_FOUND)
}
