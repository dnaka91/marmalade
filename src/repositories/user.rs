use std::{collections::HashSet, convert::TryFrom, io::ErrorKind};

use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use camino::Utf8PathBuf;
use tokio::fs;
use uuid::Uuid;

use super::RepoRepository;
use crate::{dirs::DIRS, models::UserAccount};

pub struct UserRepository<'a> {
    username: &'a str,
    pub(super) user_path: Utf8PathBuf,
    pub(super) user_file: Utf8PathBuf,
}

impl<'a> UserRepository<'a> {
    pub fn for_user(username: &'a str) -> Self {
        let user_path = DIRS.data_dir().join(username);
        let user_file = user_path.join("user.json");

        Self {
            username,
            user_path,
            user_file,
        }
    }

    pub fn repo<'b>(&self, name: &'b str) -> RepoRepository<'b> {
        RepoRepository::from_user_repo(self, name)
    }

    pub async fn exists(&self) -> bool {
        fs::metadata(&self.user_file).await.is_ok()
    }

    pub async fn visible(&self, auth_user: &str, user: &str) -> Result<bool> {
        if auth_user == user {
            return Ok(true);
        }

        Ok(!self.load_info().await?.private)
    }

    pub async fn create_user(&self, password: &str, private: bool, admin: bool) -> Result<bool> {
        if self.exists().await {
            return Ok(false);
        }

        let data = serde_json::to_vec_pretty(&UserAccount {
            username: self.username.to_owned(),
            password: hash_password(password)?,
            private,
            admin,
        })?;

        fs::create_dir_all(&self.user_path).await?;
        fs::write(&self.user_file, data).await?;

        Ok(true)
    }

    pub async fn is_valid_password(&self, password: &str) -> Result<bool> {
        let user_file = fs::read(&self.user_file).await?;

        let data = serde_json::from_slice::<UserAccount>(&user_file)?;

        verify_password(password, &data.password)
    }

    pub async fn is_valid_token(&self, token: Uuid) -> Result<bool> {
        let token_file = self.user_path.join("tokens.json");
        let token_file = fs::read(token_file).await?;

        let tokens = serde_json::from_slice::<HashSet<Uuid>>(&token_file)?;

        Ok(tokens.contains(&token))
    }

    pub async fn add_token(&self, token: Uuid) -> Result<()> {
        edit_tokens(self.username, |tokens| {
            tokens.insert(token);
        })
        .await
    }

    pub async fn remove_token(&self, token: Uuid) -> Result<()> {
        edit_tokens(self.username, |tokens| {
            tokens.remove(&token);
        })
        .await
    }

    pub async fn list_user_names(&self, auth_user: &str) -> Result<Vec<String>> {
        let mut entries = fs::read_dir(DIRS.data_dir()).await?;
        let mut names = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = Utf8PathBuf::try_from(entry.path())?;
            let file_name = path.file_name().unwrap();

            if auth_user != file_name
                && fs::metadata(path.join("user.json")).await.is_ok()
                && UserRepository::for_user(file_name)
                    .visible(auth_user, file_name)
                    .await?
            {
                names.push(file_name.to_owned());
            }
        }

        Ok(names)
    }

    pub async fn list_repo_names(&self, auth_user: &str) -> Result<Vec<String>> {
        let mut entries = fs::read_dir(&self.user_path).await?;
        let mut names = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = Utf8PathBuf::try_from(entry.path())?;

            if fs::metadata(path.join("repo.json")).await.is_ok()
                && fs::metadata(path.join("repo.git")).await.is_ok()
                && RepoRepository::from_base(&self.user_path, path.file_name().unwrap())
                    .visible(auth_user, self.username)
                    .await?
            {
                names.push(path.file_name().unwrap().to_owned());
            }
        }

        Ok(names)
    }

    async fn load_info(&self) -> Result<UserAccount> {
        let data = fs::read(&self.user_file).await?;
        serde_json::from_slice(&data).map_err(Into::into)
    }
}

async fn edit_tokens(username: &str, edit: impl Fn(&mut HashSet<Uuid>)) -> Result<()> {
    let user_path = DIRS.data_dir().join(username);
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
