use std::str::FromStr;

use anyhow::bail;
use askama::Template;
use askama_web::WebTemplate;
use camino::Utf8PathBuf;

use crate::models::{FileKind, RepoFile, RepoTree, TreeKind, UserAccount, UserRepo};

#[derive(Template, WebTemplate)]
#[template(path = "repo/index.html")]
pub struct Index {
    pub auth_user: Option<UserAccount>,
    pub user: String,
    pub repo: String,
    pub branch: String,
    pub files: Vec<RepoFile>,
    pub readme: String,
}

#[derive(Template, WebTemplate)]
#[template(path = "repo/tree.html")]
pub struct Tree {
    pub auth_user: Option<UserAccount>,
    pub user: String,
    pub repo: String,
    pub branch: String,
    pub branches: Vec<String>,
    pub path: Utf8PathBuf,
    pub tree: RepoTree,
}

impl Tree {
    fn paths(&self) -> Vec<(&str, Utf8PathBuf)> {
        let mut current = Utf8PathBuf::new();
        let mut paths = Vec::new();

        for comp in self.path.components() {
            current.push(comp.as_str());
            paths.push((comp.as_str(), current.clone()));
        }

        paths
    }

    fn path_of(&self, file: &str) -> String {
        let mut base = format!("/{}/{}/tree", self.user, self.repo);
        if !self.path.as_str().is_empty() {
            base.push('/');
            base.push_str(self.path.as_str());
        }

        base.push('/');
        base.push_str(file);

        base
    }
}

#[derive(Template, WebTemplate)]
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
            _ => bail!("unknown variant `{s}`"),
        })
    }
}

#[derive(Template, WebTemplate)]
#[template(path = "repo/delete.html")]
pub struct Delete {
    pub repo: String,
}

#[derive(Template, WebTemplate)]
#[template(path = "repo/settings.html")]
pub struct Settings {
    pub message: Option<RepoSettingsMessage>,
    pub auth_user: Option<UserAccount>,
    pub user: String,
    pub repo: String,
    pub branch: String,
    pub branches: Vec<String>,
    pub settings: UserRepo,
}

#[derive(Clone, Copy)]
pub enum RepoSettingsMessage {
    Success,
}

impl AsRef<str> for RepoSettingsMessage {
    fn as_ref(&self) -> &str {
        match *self {
            Self::Success => "RepoSettingsMessage::Success",
        }
    }
}

impl FromStr for RepoSettingsMessage {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "RepoSettingsMessage::Success" => Self::Success,
            _ => bail!("unknown variant `{s}`"),
        })
    }
}
