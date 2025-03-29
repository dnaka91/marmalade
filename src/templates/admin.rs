use std::str::FromStr;

use anyhow::bail;
use askama::Template;
use askama_web::WebTemplate;

use crate::models::UserAccount;

#[derive(Template, WebTemplate)]
#[template(path = "admin/settings.html")]
pub struct Settings {
    pub message: Option<ServerSettingsMessage>,
    pub auth_user: Option<UserAccount>,
    pub onion: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ServerSettingsMessage {
    Success,
    FailedReset,
}

impl AsRef<str> for ServerSettingsMessage {
    fn as_ref(&self) -> &str {
        match *self {
            Self::Success => "ServerSettingsMessage::Success",
            Self::FailedReset => "ServerSettingsMessage::FailedReset",
        }
    }
}

impl FromStr for ServerSettingsMessage {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "ServerSettingsMessage::Success" => Self::Success,
            "ServerSettingsMessage::FailedReset" => Self::FailedReset,
            _ => bail!("unknown variant `{s}`"),
        })
    }
}

impl PartialEq<ServerSettingsMessage> for &ServerSettingsMessage {
    fn eq(&self, other: &ServerSettingsMessage) -> bool {
        (*self).eq(other)
    }
}
