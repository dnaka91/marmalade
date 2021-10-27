#![allow(clippy::borrow_interior_mutable_const)]

use std::time::{Duration, SystemTime};

use axum::{
    extract::{Path, TypedHeader},
    http::StatusCode,
    response::IntoResponse,
};
use headers::{
    CacheControl, ContentType, HeaderMap, HeaderMapExt, IfModifiedSince, IfNoneMatch, LastModified,
};
use tracing::info;

use crate::assets;

/// Max age setting for the `Cache-Control` header, currently set to **2 years**.
const CACHE_MAX_AGE: Duration = Duration::from_secs(63_072_000);

pub async fn favicon_32() -> impl IntoResponse {
    info!("got assets favicon-32 request");
    include_bytes!("../../assets/favicon-32x32.png").as_ref()
}

pub async fn favicon_16() -> impl IntoResponse {
    info!("got assets favicon-16 request");
    include_bytes!("../../assets/favicon-16x16.png").as_ref()
}

pub async fn main_css(
    if_modified_since: Option<TypedHeader<IfModifiedSince>>,
    if_none_match: Option<TypedHeader<IfNoneMatch>>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let modified = if_modified_since.map(|v| v.is_modified(SystemTime::UNIX_EPOCH));
    let unmatched = if_none_match.map(|v| v.precondition_passes(&*assets::MAIN_CSS_HASH));

    info!(?modified, ?unmatched, "got assets main-css request");

    let mut headers = HeaderMap::with_capacity(4);
    headers.typed_insert(ContentType::from(mime::TEXT_CSS));
    headers.typed_insert(
        CacheControl::new()
            .with_public()
            .with_max_age(CACHE_MAX_AGE),
    );
    headers.typed_insert(assets::MAIN_CSS_HASH.clone());
    headers.typed_insert(LastModified::from(SystemTime::UNIX_EPOCH));

    if modified.or(unmatched).unwrap_or(true) {
        Ok((headers, assets::MAIN_CSS_CONTENT))
    } else {
        Err((headers, StatusCode::NOT_MODIFIED))
    }
}

pub async fn webfonts(
    Path(path): Path<String>,
    if_modified_since: Option<TypedHeader<IfModifiedSince>>,
    if_none_match: Option<TypedHeader<IfNoneMatch>>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    info!("got assets webfonts request");

    let index = assets::WEBFONTS_NAME
        .iter()
        .position(|&route| route == path)
        .ok_or_else(|| (HeaderMap::new(), StatusCode::NOT_FOUND))?;

    let modified = if_modified_since.map(|v| v.is_modified(SystemTime::UNIX_EPOCH));
    let unmatched = if_none_match.map(|v| v.precondition_passes(&assets::WEBFONTS_HASH[index]));

    let mut headers = HeaderMap::with_capacity(4);
    headers.typed_insert(ContentType::from(mime::FONT_WOFF2));
    headers.typed_insert(
        CacheControl::new()
            .with_public()
            .with_max_age(CACHE_MAX_AGE),
    );
    headers.typed_insert(assets::WEBFONTS_HASH[index].clone());
    headers.typed_insert(LastModified::from(SystemTime::UNIX_EPOCH));

    if modified.or(unmatched).unwrap_or(true) {
        Ok((headers, assets::WEBFONTS_CONTENT[index]))
    } else {
        Err((headers, StatusCode::NOT_MODIFIED))
    }
}
