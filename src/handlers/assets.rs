#![allow(clippy::borrow_interior_mutable_const)]

use std::time::{Duration, SystemTime};

use axum::{
    extract::{Path, TypedHeader},
    http::StatusCode,
    response::IntoResponse,
};
use headers::{
    CacheControl, ContentType, ETag, HeaderMap, HeaderMapExt, IfModifiedSince, IfNoneMatch,
    LastModified,
};
use mime::Mime;
use tracing::info;

use crate::assets;

type AssetResponse = Result<(HeaderMap, &'static [u8]), (HeaderMap, StatusCode)>;

/// Max age setting for the `Cache-Control` header, currently set to **2 years**.
const CACHE_MAX_AGE: Duration = Duration::from_secs(63_072_000);

pub async fn favicon_32(
    if_modified_since: Option<TypedHeader<IfModifiedSince>>,
    if_none_match: Option<TypedHeader<IfNoneMatch>>,
) -> AssetResponse {
    info!("got assets favicon-32 request");
    favicon(
        &*assets::FAVICON_X32_HASH,
        assets::FAVICON_X32_CONTENT,
        mime::IMAGE_PNG,
        if_modified_since,
        if_none_match,
    )
}

pub async fn favicon_16(
    if_modified_since: Option<TypedHeader<IfModifiedSince>>,
    if_none_match: Option<TypedHeader<IfNoneMatch>>,
) -> AssetResponse {
    info!("got assets favicon-16 request");
    favicon(
        &*assets::FAVICON_X16_HASH,
        assets::FAVICON_X16_CONTENT,
        mime::IMAGE_PNG,
        if_modified_since,
        if_none_match,
    )
}

pub async fn main_css(
    if_modified_since: Option<TypedHeader<IfModifiedSince>>,
    if_none_match: Option<TypedHeader<IfNoneMatch>>,
) -> impl IntoResponse {
    info!("got assets main-css request");
    favicon(
        &*assets::MAIN_CSS_HASH,
        assets::MAIN_CSS_CONTENT,
        mime::TEXT_CSS,
        if_modified_since,
        if_none_match,
    )
}

pub async fn webfonts(
    Path(path): Path<String>,
    if_modified_since: Option<TypedHeader<IfModifiedSince>>,
    if_none_match: Option<TypedHeader<IfNoneMatch>>,
) -> AssetResponse {
    info!("got assets webfonts request");

    let index = assets::WEBFONTS_NAME
        .iter()
        .position(|&route| route == path)
        .ok_or_else(|| (HeaderMap::new(), StatusCode::NOT_FOUND))?;

    favicon(
        &assets::WEBFONTS_HASH[index],
        assets::WEBFONTS_CONTENT[index],
        mime::FONT_WOFF2,
        if_modified_since,
        if_none_match,
    )
}

fn favicon(
    etag: &ETag,
    content: &'static [u8],
    mime: Mime,
    if_modified_since: Option<TypedHeader<IfModifiedSince>>,
    if_none_match: Option<TypedHeader<IfNoneMatch>>,
) -> AssetResponse {
    let modified = if_modified_since.map(|v| v.is_modified(SystemTime::UNIX_EPOCH));
    let unmatched = if_none_match.map(|v| v.precondition_passes(etag));

    let mut headers = HeaderMap::with_capacity(4);
    headers.typed_insert(ContentType::from(mime));
    headers.typed_insert(
        CacheControl::new()
            .with_public()
            .with_max_age(CACHE_MAX_AGE),
    );
    headers.typed_insert(etag.clone());
    headers.typed_insert(LastModified::from(SystemTime::UNIX_EPOCH));

    if modified.or(unmatched).unwrap_or(true) {
        Ok((headers, content))
    } else {
        Err((headers, StatusCode::NOT_MODIFIED))
    }
}
