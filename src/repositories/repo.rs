use anyhow::{Context, Result};
use futures_util::FutureExt;
use git2::{ErrorCode, ObjectType, Repository, Tree};
use tokio::fs;

use crate::{
    dirs::DIRS,
    models::{FileKind, RepoFile, UserRepo},
};

pub struct RepoRepository<'a, 'b> {
    user: &'a str,
    repo: &'b str,
}

impl<'a, 'b> RepoRepository<'a, 'b> {
    pub fn for_repo(user: &'a str, repo: &'b str) -> Self {
        Self { user, repo }
    }

    pub async fn exists(&self) -> bool {
        let (file, git) = tokio::join!(
            fs::metadata(DIRS.repo_info_file(self.user, self.repo)).map(|m| m.is_ok()),
            fs::metadata(DIRS.repo_git_dir(self.user, self.repo)).map(|m| m.is_ok())
        );

        file && git
    }

    pub async fn visible(&self, auth_user: &str, repo_user: &str) -> Result<bool> {
        if auth_user == repo_user {
            return Ok(true);
        }

        Ok(!self.load_info().await?.private)
    }

    pub async fn create(&self, private: bool) -> Result<bool> {
        if self.exists().await {
            return Ok(false);
        }

        let data = serde_json::to_vec_pretty(&UserRepo {
            name: self.repo.to_owned(),
            private,
        })?;

        fs::create_dir_all(DIRS.repo_dir(self.user, self.repo))
            .await
            .context("failed creating repo folder")?;
        fs::write(DIRS.repo_info_file(self.user, self.repo), data)
            .await
            .context("failed writing repo info file")?;

        fs::create_dir_all(DIRS.repo_git_dir(self.user, self.repo))
            .await
            .context("failed creating repo git folder")?;

        let repo_git = DIRS.repo_git_dir(self.user, self.repo);
        tokio::task::spawn_blocking(move || {
            Repository::init_bare(repo_git).context("failed initializing bare repo")
        })
        .await??;

        Ok(true)
    }

    pub async fn delete(&self) -> Result<bool> {
        if !self.exists().await {
            return Ok(false);
        }

        fs::remove_dir_all(DIRS.repo_dir(self.user, self.repo))
            .await
            .context("failed removing repo folder")?;

        Ok(true)
    }

    pub async fn get_readme(&self) -> Result<Option<String>> {
        if !self.exists().await {
            return Ok(None);
        }

        let repo_git = DIRS.repo_git_dir(self.user, self.repo);
        let readme = tokio::task::spawn_blocking(move || -> Result<Option<String>> {
            let repo = Repository::open(repo_git).context("failed opening repo")?;
            let tree = match get_head_tree(&repo).context("failed getting head commit tree")? {
                Some(tree) => tree,
                None => return Ok(None),
            };
            let entry = tree.iter().find(|entry| {
                entry
                    .name()
                    .map(|name| {
                        name.eq_ignore_ascii_case("README.md")
                            || name.eq_ignore_ascii_case("README")
                    })
                    .unwrap_or_default()
                    && entry
                        .kind()
                        .map(|kind| kind == ObjectType::Blob)
                        .unwrap_or_default()
            });

            let content = match entry {
                Some(entry) => {
                    let blob = entry
                        .to_object(&repo)
                        .context("failed converting entry to object")?
                        .into_blob()
                        .unwrap();
                    String::from_utf8(blob.content().to_owned()).ok()
                }
                None => None,
            };

            Ok(content)
        })
        .await??;

        Ok(readme)
    }

    pub async fn get_file_list(&self) -> Result<Vec<RepoFile>> {
        if !self.exists().await {
            return Ok(Vec::new());
        }

        let repo_git = DIRS.repo_git_dir(self.user, self.repo);
        let list = tokio::task::spawn_blocking(move || -> Result<Vec<RepoFile>> {
            let repo = Repository::open(repo_git).context("failed opening repo")?;
            let tree = match get_head_tree(&repo).context("failed getting head commit tree")? {
                Some(tree) => tree,
                None => return Ok(Vec::new()),
            };

            Ok(tree
                .iter()
                .filter_map(|entry| {
                    let kind = match entry.kind()? {
                        ObjectType::Tree => FileKind::Directory,
                        ObjectType::Blob => FileKind::File,
                        _ => return None,
                    };
                    let name = entry.name()?.to_owned();

                    Some(RepoFile { name, kind })
                })
                .collect())
        })
        .await??;

        Ok(list)
    }

    async fn load_info(&self) -> Result<UserRepo> {
        let data = fs::read(DIRS.repo_info_file(self.user, self.repo)).await?;
        serde_json::from_slice(&data).map_err(Into::into)
    }
}

fn get_head_tree(repo: &Repository) -> Result<Option<Tree<'_>>> {
    match repo.head() {
        Ok(head) => match head.peel_to_commit() {
            Ok(commit) => commit.tree().map(Some).map_err(Into::into),
            Err(e) if e.code() == ErrorCode::UnbornBranch => Ok(None),
            Err(e) => Err(e.into()),
        },
        Err(e) if e.code() == ErrorCode::UnbornBranch => Ok(None),
        Err(e) => Err(e.into()),
    }
}
