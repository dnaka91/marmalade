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
