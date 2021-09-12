#![allow(clippy::unused_async)]

use std::process::Stdio;

use axum::{
    body::{Body, Full},
    extract::{Path, Query},
    http::{Response, StatusCode},
    response::IntoResponse,
};
use camino::Utf8PathBuf;
use futures_util::StreamExt;
use serde::Deserialize;
use tokio::{io::AsyncWriteExt, process::Command};
use tokio_util::io::ReaderStream;
use tracing::info;

use crate::{response::HtmlTemplate, templates};

pub async fn hello() -> impl IntoResponse {
    HtmlTemplate(templates::Index)
}

pub async fn favicon_32() -> impl IntoResponse {
    include_bytes!("../assets/favicon-32x32.png").as_ref()
}

pub async fn favicon_16() -> impl IntoResponse {
    include_bytes!("../assets/favicon-16x16.png").as_ref()
}

#[derive(Debug, Deserialize)]
pub struct GitInfoRefsParams {
    user: String,
    repo: String,
}

#[derive(Debug, Deserialize)]
pub struct GitInfoRefsQuery {
    service: GitService,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GitService {
    GitReceivePack,
    GitUploadPack,
}

pub async fn git_info_refs(
    Path(params): Path<GitInfoRefsParams>,
    Query(query): Query<GitInfoRefsQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    const BODY_HEADER_RECEIVE: &str = "001f# service=git-receive-pack\n0000";
    const BODY_HEADER_UPLOAD: &str = "001e# service=git-upload-pack\n0000";

    info!(user = ?params.user, repo = ?params.repo, "got request");

    let path = Utf8PathBuf::from(format!(
        "temp/{}/{}",
        params.user,
        params.repo.strip_suffix(".git").unwrap_or(&params.repo)
    ));

    if tokio::fs::metadata(&path).await.is_err() {
        return Err(StatusCode::NOT_FOUND);
    }

    let command = match query.service {
        GitService::GitReceivePack => "git-receive-pack",
        GitService::GitUploadPack => "git-upload-pack",
    };

    let output = Command::new(command)
        .arg("--advertise-refs")
        .arg(path)
        .output()
        .await
        .unwrap();

    let header = match query.service {
        GitService::GitReceivePack => BODY_HEADER_RECEIVE,
        GitService::GitUploadPack => BODY_HEADER_UPLOAD,
    };

    let mut body = Vec::with_capacity(output.stdout.len() + header.len());
    body.extend(header.as_bytes());
    body.extend(output.stdout);

    let temp = String::from_utf8_lossy(&body);
    info!("ran {}:\n{}", command, temp);

    let content_type = match query.service {
        GitService::GitReceivePack => "application/x-git-receive-pack-advertisement",
        GitService::GitUploadPack => "application/x-git-upload-pack-advertisement",
    };

    Ok(Response::builder()
        .header("Content-Type", content_type)
        .header("Cache-Control", "no-cache")
        .body(Full::from(body))
        .unwrap())
}

pub async fn git_receive_pack(
    Path(params): Path<GitInfoRefsParams>,
    mut body: Body,
) -> Result<impl IntoResponse, StatusCode> {
    info!(user = ?params.user, repo = ?params.repo, "got request");

    let path = Utf8PathBuf::from(format!(
        "temp/{}/{}",
        params.user,
        params.repo.strip_suffix(".git").unwrap_or(&params.repo)
    ));

    if tokio::fs::metadata(&path).await.is_err() {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut process = Command::new("git-receive-pack")
        .arg("--stateless-rpc")
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = process.stdin.take().unwrap();
    let stdout = process.stdout.take().unwrap();

    tokio::spawn(async move {
        while let Some(value) = body.next().await {
            let mut value = value.unwrap();

            stdin.write_all_buf(&mut value).await.unwrap();
        }

        process.wait().await.unwrap();
    });

    let body = Body::wrap_stream(ReaderStream::new(stdout));

    Ok(Response::builder()
        .header("Content-Type", "application/x-git-receive-pack-result")
        .header("Cache-Control", "no-cache")
        .body(body)
        .unwrap())
}

pub async fn git_upload_pack(
    Path(params): Path<GitInfoRefsParams>,
    mut body: Body,
) -> Result<impl IntoResponse, StatusCode> {
    info!(user = ?params.user, repo = ?params.repo, "got request");

    let path = Utf8PathBuf::from(format!(
        "temp/{}/{}",
        params.user,
        params.repo.strip_suffix(".git").unwrap_or(&params.repo)
    ));

    if tokio::fs::metadata(&path).await.is_err() {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut process = Command::new("git-upload-pack")
        .arg("--stateless-rpc")
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = process.stdin.take().unwrap();
    let stdout = process.stdout.take().unwrap();

    tokio::spawn(async move {
        while let Some(value) = body.next().await {
            let mut value = value.unwrap();

            stdin.write_all_buf(&mut value).await.unwrap();
        }

        process.wait().await.unwrap();
    });

    let body = Body::wrap_stream(ReaderStream::new(stdout));

    Ok(Response::builder()
        .header("Content-Type", "application/x-git-upload-pack-result")
        .header("Cache-Control", "no-cache")
        .body(body)
        .unwrap())
}
