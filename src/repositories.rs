use std::{collections::HashSet, io::ErrorKind, sync::Arc};

use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use camino::{Utf8Path, Utf8PathBuf};
use futures_util::FutureExt;
use git2::Repository;
use tokio::fs;
use uuid::Uuid;

use crate::{
    dirs::DIRS,
    models::{UserAccount, UserRepo},
};

pub struct UserRepository<'a> {
    username: &'a str,
    user_path: Utf8PathBuf,
    user_file: Utf8PathBuf,
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

    pub async fn create_user(&self, password: &str, admin: bool) -> Result<bool> {
        if self.exists().await {
            return Ok(false);
        }

        let data = serde_json::to_vec_pretty(&UserAccount {
            username: self.username.to_owned(),
            password: hash_password(password)?,
            admin,
        })?;

        fs::create_dir_all(&self.user_path).await?;
        fs::write(&self.user_file, data).await?;

        Ok(true)
    }

    pub async fn is_valid_password(&self, password: &str) -> Result<bool> {
        let user_file = self.user_path.join("user.json");
        let user_file = fs::read(user_file).await?;

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

pub struct RepoRepository<'a> {
    name: &'a str,
    repo_path: Utf8PathBuf,
    repo_file: Utf8PathBuf,
    repo_git: Arc<Utf8PathBuf>,
}

impl<'a> RepoRepository<'a> {
    pub fn new(user: &str, repo: &'a str) -> Self {
        Self::from_base(&DIRS.data_dir().join(user), repo)
    }

    fn from_user_repo(user_repo: &UserRepository<'_>, name: &'a str) -> Self {
        Self::from_base(&user_repo.user_path, name)
    }

    fn from_base(base: &Utf8Path, name: &'a str) -> Self {
        let repo_path = base.join(name);
        let repo_file = repo_path.join("repo.json");
        let repo_git = Arc::new(repo_path.join("repo.git"));

        Self {
            name,
            repo_path,
            repo_file,
            repo_git,
        }
    }

    pub async fn exists(&self) -> bool {
        let (file, git) = tokio::join!(
            fs::metadata(&self.repo_file).map(|m| m.is_ok()),
            fs::metadata(&*self.repo_git).map(|m| m.is_ok())
        );

        file && git
    }

    pub async fn create(&self, private: bool) -> Result<bool> {
        if self.exists().await {
            return Ok(false);
        }

        let data = serde_json::to_vec_pretty(&UserRepo {
            name: self.name.to_owned(),
            private,
        })?;

        fs::create_dir_all(&self.repo_path).await?;
        fs::write(&self.repo_file, data).await?;

        fs::create_dir_all(&*self.repo_git).await?;

        let repo_git = Arc::clone(&self.repo_git);
        tokio::task::spawn_blocking(move || Repository::init_bare(&*repo_git));

        Ok(true)
    }
}
