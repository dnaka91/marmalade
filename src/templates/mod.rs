#![allow(clippy::struct_field_names)]

use std::str::FromStr;

use anyhow::bail;
use axum::http::StatusCode;
use rinja::Template;

use crate::models::UserAccount;

pub mod admin;
pub mod repo;
pub mod user;

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index {
    pub auth_user: Option<UserAccount>,
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
            _ => bail!("unknown variant `{s}`"),
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
            _ => bail!("unknown variant `{s}`"),
        })
    }
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
