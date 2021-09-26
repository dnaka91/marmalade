use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct UserAccount {
    pub username: String,
    pub password: String,
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
