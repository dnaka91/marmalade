use std::{
    io::{Error as IoError, ErrorKind},
    process::Stdio,
};

use axum::{
    body::{Body, Full},
    extract::{Path, Query},
    http::{Response, StatusCode},
    response::IntoResponse,
};
use futures_util::TryStreamExt;
use serde::Deserialize;
use tokio::{fs, process::Command};
use tokio_util::io::{ReaderStream, StreamReader};
use tracing::{error, info};

use crate::dirs::DIRS;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GitService {
    GitReceivePack,
    GitUploadPack,
}

impl GitService {
    const fn command(self) -> &'static str {
        match self {
            Self::GitReceivePack => "git-receive-pack",
            Self::GitUploadPack => "git-upload-pack",
        }
    }

    const fn advertise_header(self) -> &'static str {
        match self {
            Self::GitReceivePack => "001f# service=git-receive-pack\n0000",
            Self::GitUploadPack => "001e# service=git-upload-pack\n0000",
        }
    }

    const fn content_type(self, advertise: bool) -> &'static str {
        if advertise {
            match self {
                Self::GitReceivePack => "application/x-git-receive-pack-advertisement",
                Self::GitUploadPack => "application/x-git-upload-pack-advertisement",
            }
        } else {
            match self {
                Self::GitReceivePack => "application/x-git-receive-pack-result",
                Self::GitUploadPack => "application/x-git-upload-pack-result",
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct InfoRefsParams {
    #[serde(deserialize_with = "crate::de::percent")]
    user: String,
    #[serde(deserialize_with = "crate::de::percent")]
    repo: String,
}

#[derive(Debug, Deserialize)]
pub struct InfoRefsQuery {
    service: GitService,
}

pub async fn info_refs(
    Path(params): Path<InfoRefsParams>,
    Query(query): Query<InfoRefsQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    info!(user = ?params.user, repo = ?params.repo, "got request");

    let path = DIRS
        .data_dir()
        .join(&params.user)
        .join(params.repo.strip_suffix(".git").unwrap_or(&params.repo))
        .join("repo.git");

    if fs::metadata(&path).await.is_err() {
        return Err(StatusCode::NOT_FOUND);
    }

    let output = Command::new(query.service.command())
        .arg("--advertise-refs")
        .arg(path)
        .output()
        .await
        .map_err(|error| {
            error!(command=?query.service.command(), ?error, "failed running command");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let header = query.service.advertise_header();
    let mut body = Vec::with_capacity(header.len() + output.stdout.len());
    body.extend(header.as_bytes());
    body.extend(output.stdout);

    Ok(Response::builder()
        .header("Content-Type", query.service.content_type(true))
        .header("Cache-Control", "no-cache")
        .body(Full::from(body))
        .unwrap())
}

#[derive(Debug, Deserialize)]
pub struct PackParams {
    #[serde(deserialize_with = "crate::de::percent")]
    user: String,
    #[serde(deserialize_with = "crate::de::percent")]
    repo: String,
    service: GitService,
}

pub async fn pack(
    Path(params): Path<PackParams>,
    body: Body,
) -> Result<impl IntoResponse, StatusCode> {
    info!(user = ?params.user, repo = ?params.repo, "got request");

    let path = DIRS
        .data_dir()
        .join(&params.user)
        .join(params.repo.strip_suffix(".git").unwrap_or(&params.repo))
        .join("repo.git");

    if fs::metadata(&path).await.is_err() {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut process = Command::new(params.service.command())
        .arg("--stateless-rpc")
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| {
            error!(command = ?params.service.command(),?error,"failed spawning command");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Unwrap: safe to unwrap as we configured stdin & stdout as piped,
    // thus being always present.
    let mut stdin = process.stdin.take().unwrap();
    let stdout = process.stdout.take().unwrap();

    tokio::spawn(async move {
        let body = body.map_err(|e| IoError::new(ErrorKind::Other, e));
        let mut body = StreamReader::new(body);

        if let Err(error) = tokio::io::copy(&mut body, &mut stdin).await {
            error!(?error, "failed copying request body to command");
            return;
        }

        if let Err(error) = process.wait().await {
            error!(?error, "failed completing command");
        }
    });

    let body = Body::wrap_stream(ReaderStream::new(stdout));

    Ok(Response::builder()
        .header("Content-Type", params.service.content_type(false))
        .header("Cache-Control", "no-cache")
        .body(body)
        .unwrap())
}
