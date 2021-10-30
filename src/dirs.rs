use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use directories_next::ProjectDirs;
use once_cell::sync::Lazy;

// Unwrap: We can't run the server without known where to place files, so panic here as there is no
// good recovery case other than throwing an error and shutting down.
pub static DIRS: Lazy<Utf8ProjectDirs> = Lazy::new(|| Utf8ProjectDirs::new().unwrap());

pub struct Utf8ProjectDirs {
    data_dir: Utf8PathBuf,
    users_dir: Utf8PathBuf,
}

impl Utf8ProjectDirs {
    fn new() -> Result<Self> {
        let data_dir = if whoami::username() == "marmalade" {
            Utf8PathBuf::from(concat!("/var/lib/", env!("CARGO_PKG_NAME")))
        } else {
            let dirs = ProjectDirs::from("rocks", "dnaka91", env!("CARGO_PKG_NAME"))
                .context("failed finding project dirs")?;

            Utf8Path::from_path(dirs.data_dir())
                .context("project data dir is not valid UTF-8")?
                .to_owned()
        };
        let users_dir = data_dir.join("users");

        Ok(Self {
            data_dir,
            users_dir,
        })
    }

    // <data>/state.json
    #[inline]
    pub fn settings_file(&self) -> Utf8PathBuf {
        self.data_dir.join("settings.json")
    }

    // <data>/~state.json
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
