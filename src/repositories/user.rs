use std::{collections::HashSet, convert::TryFrom, io::ErrorKind};

use anyhow::Result;
use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use camino::Utf8PathBuf;
use tokio::fs;
use tracing::instrument;
use uuid::Uuid;

use super::RepoRepository;
use crate::{dirs::DIRS, models::UserAccount};

pub struct UserRepository<'a> {
    user: &'a str,
}

impl<'a> UserRepository<'a> {
    pub fn for_user(user: &'a str) -> Self {
        Self { user }
    }

    pub fn repo<'b>(&self, name: &'b str) -> RepoRepository<'a, 'b> {
        RepoRepository::for_repo(self.user, name)
    }

    #[instrument(skip_all)]
    pub async fn exists(&self) -> bool {
        fs::metadata(DIRS.user_info_file(self.user)).await.is_ok()
    }

    #[instrument(skip_all)]
    pub async fn visible(&self, auth_user: &str, user: &str) -> Result<bool> {
        if auth_user == user {
            return Ok(true);
        }

        Ok(!self.load_info().await?.private)
    }

    #[instrument(skip_all)]
    pub async fn create_user(&self, info: CreateUser<'_>) -> Result<bool> {
        if self.exists().await {
            return Ok(false);
        }

        let data = serde_json::to_vec_pretty(&UserAccount {
            username: self.user.to_owned(),
            password: hash_password(info.password)?,
            description: info.description.unwrap_or_default().to_owned(),
            private: info.private,
            admin: info.admin,
        })?;

        fs::create_dir_all(DIRS.user_dir(self.user)).await?;
        fs::create_dir_all(DIRS.user_repos_dir(self.user)).await?;
        fs::write(DIRS.user_info_file(self.user), data).await?;

        Ok(true)
    }

    #[instrument(skip_all)]
    pub async fn is_valid_password(&self, password: &str) -> Result<bool> {
        let user_file = fs::read(DIRS.user_info_file(self.user)).await?;

        let data = serde_json::from_slice::<UserAccount>(&user_file)?;

        verify_password(password, &data.password)
    }

    #[instrument(skip_all)]
    pub async fn is_valid_token(&self, token: Uuid) -> Result<bool> {
        let token_file = fs::read(DIRS.user_tokens_file(self.user)).await?;

        let tokens = serde_json::from_slice::<HashSet<Uuid>>(&token_file)?;

        Ok(tokens.contains(&token))
    }

    #[instrument(skip_all)]
    pub async fn add_token(&self, token: Uuid) -> Result<()> {
        self.edit_tokens(|tokens| {
            tokens.insert(token);
        })
        .await
    }

    #[instrument(skip_all)]
    pub async fn remove_token(&self, token: Uuid) -> Result<()> {
        self.edit_tokens(|tokens| {
            tokens.remove(&token);
        })
        .await
    }

    #[instrument(skip_all)]
    pub async fn list_user_names(&self, auth_user: &str) -> Result<Vec<String>> {
        let mut entries = fs::read_dir(DIRS.users_dir()).await?;
        let mut names = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = Utf8PathBuf::try_from(entry.path())?;
            let file_name = path.file_name().unwrap();

            let user_repo = UserRepository::for_user(file_name);

            if user_repo.exists().await && user_repo.visible(auth_user, file_name).await? {
                names.push(file_name.to_owned());
            }
        }

        Ok(names)
    }

    #[instrument(skip_all)]
    pub async fn list_repo_names(&self, auth_user: &str) -> Result<Vec<(String, String)>> {
        let mut entries = fs::read_dir(DIRS.user_repos_dir(self.user)).await?;
        let mut names = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = Utf8PathBuf::try_from(entry.path())?;
            let file_name = path.file_name().unwrap();

            let repo_repo = self.repo(file_name);

            if repo_repo.exists().await && repo_repo.visible(auth_user, self.user).await? {
                let description = repo_repo.load_info().await.unwrap().description;
                names.push((path.file_name().unwrap().to_owned(), description));
            }
        }

        Ok(names)
    }

    #[instrument(skip_all)]
    async fn edit_tokens(&self, edit: impl Fn(&mut HashSet<Uuid>)) -> Result<()> {
        let real_file = DIRS.user_tokens_file(self.user);
        let temp_file = DIRS.user_tokens_temp_file(self.user);

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

    #[instrument(skip_all)]
    pub async fn load_info(&self) -> Result<UserAccount> {
        let data = fs::read(DIRS.user_info_file(self.user)).await?;
        serde_json::from_slice(&data).map_err(Into::into)
    }

    #[instrument(skip_all)]
    pub async fn save_info(&self, info: &UserAccount) -> Result<()> {
        let real_file = DIRS.user_info_file(self.user);
        let temp_file = DIRS.user_info_temp_file(self.user);

        let buf = serde_json::to_vec_pretty(info)?;
        fs::write(&temp_file, &buf).await?;
        fs::rename(temp_file, real_file).await?;

        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn change_password(&self, password: &str) -> Result<()> {
        let mut info = self.load_info().await?;
        info.password = hash_password(password)?;

        self.save_info(&info).await?;
        self.clear_tokens().await
    }

    #[instrument(skip_all)]
    pub async fn clear_tokens(&self) -> Result<()> {
        self.edit_tokens(HashSet::clear).await
    }
}

pub struct CreateUser<'a> {
    pub password: &'a str,
    pub description: Option<&'a str>,
    pub private: bool,
    pub admin: bool,
}

#[instrument(skip_all)]
fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hasher = Argon2::default();

    Ok(hasher
        .hash_password(password.as_bytes(), &salt)?
        .to_string())
}

#[instrument(skip_all)]
fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let hash = PasswordHash::new(hash)?;
    let hasher = Argon2::default();

    Ok(hasher.verify_password(password.as_bytes(), &hash).is_ok())
}
