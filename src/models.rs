use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct UserAccount {
    pub username: String,
    pub password: String,
    pub admin: bool,
}
