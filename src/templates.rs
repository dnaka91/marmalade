use std::str::FromStr;

use anyhow::bail;
use askama::Template;
use axum::http::StatusCode;

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index {
    pub auth_user: Option<String>,
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct Login {
    pub error: Option<LoginError>,
}

#[derive(Clone, Copy)]
pub enum LoginError {
    Empty,
    UnknownUser,
}

impl AsRef<str> for LoginError {
    fn as_ref(&self) -> &str {
        match *self {
            Self::Empty => "LoginError::Empty",
            Self::UnknownUser => "LoginError::UnknownUser",
        }
    }
}

impl FromStr for LoginError {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "LoginError::Empty" => Self::Empty,
            "LoginError::UnknownUser" => Self::UnknownUser,
            _ => bail!("unknown variant `{}`", s),
        })
    }
}

#[derive(Template)]
#[template(path = "register.html")]
pub struct Register {
    pub error: Option<RegisterError>,
}

#[derive(Clone, Copy)]
pub enum RegisterError {
    InvalidUsername,
    InvalidPassword,
    UsernameTaken,
}

impl AsRef<str> for RegisterError {
    fn as_ref(&self) -> &str {
        match *self {
            Self::InvalidUsername => "RegisterError::InvalidUsername",
            Self::InvalidPassword => "RegisterError::InvalidPassword",
            Self::UsernameTaken => "RegisterError::UsernameTaken",
        }
    }
}

impl FromStr for RegisterError {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "RegisterError::InvalidUsername" => Self::InvalidUsername,
            "RegisterError::InvalidPassword" => Self::InvalidPassword,
            "RegisterError::UsernameTaken" => Self::UsernameTaken,
            _ => bail!("unknown variant `{}`", s),
        })
    }
}

#[derive(Template)]
#[template(path = "show.html")]
pub struct Show {
    pub username: Option<String>,
}

#[derive(Template)]
#[template(path = "error.html")]
pub struct Error {
    pub code: StatusCode,
    pub message: Option<&'static str>,
}

#[allow(clippy::trivially_copy_pass_by_ref)]
pub fn status_emoji(status: &StatusCode) -> char {
    match *status {
        StatusCode::NOT_FOUND => '🤷',
        StatusCode::FORBIDDEN => '🙅',
        _ => ' ',
    }
}

pub mod user {
    use askama::Template;

    #[derive(Template)]
    #[template(path = "user/index.html")]
    pub struct Index {
        pub auth_user: Option<String>,
        pub user: String,
        pub repos: Vec<String>,
    }

    impl Index {
        fn auth_same_user(&self) -> bool {
            self.auth_user
                .as_deref()
                .map(|u| u == self.user)
                .unwrap_or_default()
        }
    }

    #[derive(Template)]
    #[template(path = "user/list.html")]
    pub struct List {
        pub auth_user: Option<String>,
        pub users: Vec<String>,
    }
}

pub mod repo {
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
}
