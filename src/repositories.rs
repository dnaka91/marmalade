use std::{collections::HashSet, io::ErrorKind};

use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use camino::Utf8Path;
use tokio::fs;
use uuid::Uuid;

use crate::models::UserAccount;

const BASE_PATH: &str = "temp";

pub struct UserRepository;

impl UserRepository {
    pub async fn create_user(&self, username: &str, password: &str, admin: bool) -> Result<bool> {
        let user_path = Utf8Path::new(BASE_PATH).join(username);
        let user_file = Utf8Path::new(BASE_PATH).join(username).join("user.json");

        if fs::metadata(&user_file).await.is_ok() {
            return Ok(false);
        }

        let data = serde_json::to_vec_pretty(&UserAccount {
            username: username.to_owned(),
            password: hash_password(password)?,
            admin,
        })?;

        fs::create_dir_all(user_path).await?;
        fs::write(user_file, data).await?;

        Ok(true)
    }

    pub async fn is_valid_password(&self, username: &str, password: &str) -> Result<bool> {
        let user_file = Utf8Path::new(BASE_PATH).join(username).join("user.json");
        let user_file = fs::read(user_file).await?;

        let data = serde_json::from_slice::<UserAccount>(&user_file)?;

        verify_password(password, &data.password)
    }

    pub async fn is_valid_token(&self, username: &str, token: Uuid) -> Result<bool> {
        let token_file = Utf8Path::new(BASE_PATH).join(username).join("tokens.json");
        let token_file = fs::read(token_file).await?;

        let tokens = serde_json::from_slice::<HashSet<Uuid>>(&token_file)?;

        Ok(tokens.contains(&token))
    }

    pub async fn add_token(&self, username: &str, token: Uuid) -> Result<()> {
        edit_tokens(username, |tokens| {
            tokens.insert(token);
        })
        .await
    }

    pub async fn remove_token(&self, username: &str, token: Uuid) -> Result<()> {
        edit_tokens(username, |tokens| {
            tokens.remove(&token);
        })
        .await
    }
}

async fn edit_tokens(username: &str, edit: impl Fn(&mut HashSet<Uuid>)) -> Result<()> {
    let user_path = Utf8Path::new(BASE_PATH).join(username);
    let real_file = user_path.join("tokens.json");
    let temp_file = user_path.join("~tokens.json");

    let mut tokens = match fs::read(&real_file).await {
        Ok(buf) => serde_json::from_slice::<HashSet<Uuid>>(&buf)?,
        Err(e) if e.kind() == ErrorKind::NotFound => HashSet::default(),
        Err(e) => return Err(e.into()),
    };

    edit(&mut tokens);

    let buf = serde_json::to_vec_pretty(&tokens)?;
    fs::write(&temp_file, &buf).await?;
    fs::rename(temp_file, real_file).await?;

    Ok(())
}

fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hasher = Argon2::default();

    Ok(hasher
        .hash_password(password.as_bytes(), &salt)?
        .to_string())
}

fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let hash = PasswordHash::new(hash)?;
    let hasher = Argon2::default();

    Ok(hasher.verify_password(password.as_bytes(), &hash).is_ok())
}
