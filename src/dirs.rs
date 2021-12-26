use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use once_cell::sync::Lazy;
use unidirs::{Directories, UnifiedDirs};

// Unwrap: We can't run the server without knowning where to place files, so panic here as there is
// no good recovery case other than throwing an error and shutting down.
pub static DIRS: Lazy<Dirs> = Lazy::new(|| Dirs::new().unwrap());

pub struct Dirs {
    data_dir: Utf8PathBuf,
    users_dir: Utf8PathBuf,
}

impl Dirs {
    fn new() -> Result<Self> {
        let data_dir = UnifiedDirs::simple("rocks", "dnaka91", env!("CARGO_PKG_NAME"))
            .default()
            .context("failed finding project directories")?
            .data_dir()
            .to_owned();
        let users_dir = data_dir.join("users");

        Ok(Self {
            data_dir,
            users_dir,
        })
    }

    // <data>
    #[inline]
    pub fn data_dir(&self) -> &Utf8Path {
        &self.data_dir
    }

    // <data>/settings.json
    #[inline]
    pub fn settings_file(&self) -> Utf8PathBuf {
        self.data_dir.join("settings.json")
    }

    // <data>/~settings.json
    #[inline]
    pub fn settings_temp_file(&self) -> Utf8PathBuf {
        self.data_dir.join("~settings.json")
    }

    // <data>/users
    #[inline]
    pub fn users_dir(&self) -> &Utf8Path {
        &self.users_dir
    }

    // <data>/users/<user>/
    #[inline]
    pub fn user_dir(&self, user: &str) -> Utf8PathBuf {
        self.users_dir.join(user)
    }

    // <data>/users/<user>/user.json
    #[inline]
    pub fn user_info_file(&self, user: &str) -> Utf8PathBuf {
        let mut dir = self.user_dir(user);
        dir.push("user.json");
        dir
    }

    // <data>/users/<user>/~user.json
    #[inline]
    pub fn user_info_temp_file(&self, user: &str) -> Utf8PathBuf {
        let mut dir = self.user_dir(user);
        dir.push("~user.json");
        dir
    }

    // <data>/users/<user>/tokens.json
    #[inline]
    pub fn user_tokens_file(&self, user: &str) -> Utf8PathBuf {
        let mut dir = self.user_dir(user);
        dir.push("tokens.json");
        dir
    }

    // <data>/users/<user>/~tokens.json
    #[inline]
    pub fn user_tokens_temp_file(&self, user: &str) -> Utf8PathBuf {
        let mut dir = self.user_dir(user);
        dir.push("~tokens.json");
        dir
    }

    // <data>/users/<user>/repos/
    #[inline]
    pub fn user_repos_dir(&self, user: &str) -> Utf8PathBuf {
        let mut dir = self.user_dir(user);
        dir.push("repos");
        dir
    }

    // <data>/users/<user>/repos/<repo>/
    #[inline]
    pub fn repo_dir(&self, user: &str, repo: &str) -> Utf8PathBuf {
        let mut dir = self.user_repos_dir(user);
        dir.push(repo);
        dir
    }

    // <data>/users/<user>/repos/<repo>/repo.json
    #[inline]
    pub fn repo_info_file(&self, user: &str, repo: &str) -> Utf8PathBuf {
        let mut dir = self.repo_dir(user, repo);
        dir.push("repo.json");
        dir
    }

    // <data>/users/<user>/repos/<repo>/~repo.json
    #[inline]
    pub fn repo_info_temp_file(&self, user: &str, repo: &str) -> Utf8PathBuf {
        let mut dir = self.repo_dir(user, repo);
        dir.push("~repo.json");
        dir
    }

    // <data>/users/<user>/repos/<repo>/repo.git/
    #[inline]
    pub fn repo_git_dir(&self, user: &str, repo: &str) -> Utf8PathBuf {
        let mut dir = self.repo_dir(user, repo);
        dir.push("repo.git");
        dir
    }
}
