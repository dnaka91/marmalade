use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct UserAccount {
    pub username: String,
    pub password: String,
    pub admin: bool,
}

#[derive(Serialize, Deserialize)]
pub struct UserRepo {
    pub name: String,
    pub private: bool,
}
