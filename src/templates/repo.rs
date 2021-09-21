use std::str::FromStr;

use anyhow::bail;
use askama::Template;

use crate::models::{FileKind, RepoFile};

#[derive(Template)]
#[template(path = "repo/index.html")]
pub struct Index {
    pub auth_user: Option<String>,
    pub user: String,
    pub repo: String,
    pub files: Vec<RepoFile>,
    pub readme: String,
}

#[derive(Template)]
#[template(path = "repo/create.html")]
pub struct Create {
    pub error: Option<RepoCreateError>,
}

#[derive(Clone, Copy)]
pub enum RepoCreateError {
    InvalidName,
    AlreadyExists,
}

impl AsRef<str> for RepoCreateError {
    fn as_ref(&self) -> &str {
        match *self {
            Self::InvalidName => "RepoCreateError::InvalidName",
            Self::AlreadyExists => "RepoCreateError::AlreadyExists",
        }
    }
}

impl FromStr for RepoCreateError {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "RepoCreateError::InvalidName" => Self::InvalidName,
            "RepoCreateError::AlreadyExists" => Self::AlreadyExists,
            _ => bail!("unknown variant `{}`", s),
        })
    }
}

#[derive(Template)]
#[template(path = "repo/delete.html")]
pub struct Delete {
    pub repo: String,
}
