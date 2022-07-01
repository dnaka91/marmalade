use std::str::FromStr;

use anyhow::bail;
use askama::Template;

use crate::models::UserAccount;

#[derive(Template)]
#[template(path = "user/index.html")]
pub struct Index {
    pub auth_user: Option<UserAccount>,
    pub user: String,
    pub repos: Vec<(String, String)>,
}

impl Index {
    fn auth_same_user(&self) -> bool {
        self.auth_user
            .as_ref()
            .map(|u| u.username == self.user)
            .unwrap_or_default()
    }
}

#[derive(Template)]
#[template(path = "user/list.html")]
pub struct List {
    pub auth_user: Option<UserAccount>,
    pub users: Vec<String>,
}

#[derive(Template)]
#[template(path = "user/settings.html")]
pub struct Settings {
    pub message: Option<UserSettingsMessage>,
    pub auth_user: Option<UserAccount>,
    pub user: String,
    pub settings: UserAccount,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UserSettingsMessage {
    Success,
    InvalidPassword,
}

impl AsRef<str> for UserSettingsMessage {
    fn as_ref(&self) -> &str {
        match *self {
            Self::Success => "UserSettingsMessage::Success",
            Self::InvalidPassword => "UserSettingsMessage::InvalidPassword",
        }
    }
}

impl FromStr for UserSettingsMessage {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "UserSettingsMessage::Success" => Self::Success,
            "UserSettingsMessage::InvalidPassword" => Self::InvalidPassword,
            _ => bail!("unknown variant `{s}`"),
        })
    }
}

impl PartialEq<UserSettingsMessage> for &UserSettingsMessage {
    fn eq(&self, other: &UserSettingsMessage) -> bool {
        (*self).eq(other)
    }
}
