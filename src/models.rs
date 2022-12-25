use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Settings {
    #[serde(with = "crate::ser::hex")]
    pub key: [u8; 64],
    pub tor: Option<Tor>,
    pub tracing: Option<Tracing>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            key: [0; 64],
            tor: None,
            tracing: None,
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct Tor {
    pub onion: String,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Tracing {
    pub archer: Option<Archer>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Archer {
    pub address: String,
    pub certificate: String,
}

#[derive(Serialize, Deserialize)]
pub struct UserAccount {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub description: String,
    pub private: bool,
    pub admin: bool,
}

#[derive(Serialize, Deserialize)]
pub struct UserRepo {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub private: bool,
}

pub struct RepoFile {
    pub name: String,
    pub kind: FileKind,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FileKind {
    Directory,
    File,
}

pub struct RepoTree {
    pub name: String,
    pub kind: TreeKind,
}

pub enum TreeKind {
    Directory(Vec<RepoFile>),
    Text(String),
    Binary(usize),
}
