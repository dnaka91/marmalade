use std::{borrow::ToOwned, str};

use anyhow::{Context, Result};
use camino::Utf8Path;
use futures_util::FutureExt;
use git2::{Blob, BranchType, ErrorCode, ObjectType, Repository, Tree};
use tokio::fs;

use crate::{
    dirs::DIRS,
    models::{FileKind, RepoFile, RepoTree, TreeKind, UserRepo},
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

    pub async fn get_branch(&self) -> Result<String> {
        if !self.exists().await {
            return Ok("master".to_owned());
        }

        let repo_git = DIRS.repo_git_dir(self.user, self.repo);
        let branch = tokio::task::spawn_blocking(move || -> Result<_> {
            let repo = Repository::open(&repo_git).context("failed opening repo")?;
            let head = match repo.head() {
                Ok(head) => {
                    let name = head.name().unwrap();
                    name.strip_prefix("refs/heads/").unwrap_or(name).to_owned()
                }
                Err(e) if e.code() == ErrorCode::UnbornBranch => {
                    let head = std::fs::read_to_string(repo_git.join("HEAD"))?;
                    match head.strip_prefix("ref: refs/heads/") {
                        Some(head) => head.to_owned(),
                        None => head,
                    }
                }
                Err(e) => return Err(e.into()),
            };

            Ok(head)
        })
        .await??;

        Ok(branch)
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
                    str::from_utf8(blob.content()).ok().map(ToOwned::to_owned)
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

        let list = tokio::task::spawn_blocking(move || -> Result<_> {
            let repo = Repository::open(repo_git).context("failed opening repo")?;
            let tree = get_head_tree(&repo).context("failed getting head commit tree")?;
            let tree = match tree {
                Some(tree) => tree,
                None => return Ok(Vec::new()),
            };

            Ok(repo_files_from_tree(&tree))
        })
        .await??;

        Ok(list)
    }

    pub async fn get_tree_list(
        &self,
        branch: &str,
        path: Option<&Utf8Path>,
    ) -> Result<Option<RepoTree>> {
        if !self.exists().await {
            return Ok(None);
        }

        let repo_git = DIRS.repo_git_dir(self.user, self.repo);
        let branch = branch.to_owned();
        let path = path.map(ToOwned::to_owned);

        let list = tokio::task::spawn_blocking(move || -> Result<_> {
            let repo = Repository::open(repo_git).context("failed opening repo")?;
            let tree = match get_branch_tree(&repo, &branch)
                .context("failed getting branch commit tree")?
            {
                Some(tree) => tree,
                None => return Ok(None),
            };

            let tree = match path {
                Some(path) => match tree.get_path(path.as_std_path()) {
                    Ok(entry) => {
                        let name = entry.name().unwrap();
                        let object = entry
                            .to_object(&repo)
                            .context("failed converting tree entry into an object")?;

                        if let Some(tree) = object.as_tree() {
                            repo_tree_from_tree(name, tree)
                        } else if let Some(blob) = object.as_blob() {
                            repo_tree_from_blob(name, blob)
                        } else {
                            return Ok(None);
                        }
                    }
                    Err(e) if e.code() == ErrorCode::NotFound => return Ok(None),
                    Err(e) => return Err(e.into()),
                },
                None => RepoTree {
                    name: "/".to_owned(),
                    kind: TreeKind::Directory(repo_files_from_tree(&tree)),
                },
            };

            Ok(Some(tree))
        })
        .await??;

        Ok(list)
    }

    async fn load_info(&self) -> Result<UserRepo> {
        let data = fs::read(DIRS.repo_info_file(self.user, self.repo)).await?;
        serde_json::from_slice(&data).map_err(Into::into)
    }
}

fn get_branch_tree<'a>(repo: &'a Repository, branch: &str) -> Result<Option<Tree<'a>>> {
    match repo.find_branch(branch, BranchType::Local) {
        Ok(branch) => match branch.into_reference().peel_to_commit() {
            Ok(commit) => commit.tree().map(Some).map_err(Into::into),
            Err(e) => Err(e.into()),
        },
        Err(e) if e.code() == ErrorCode::NotFound => Ok(None),
        Err(e) => Err(e.into()),
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

fn repo_files_from_tree(tree: &Tree<'_>) -> Vec<RepoFile> {
    tree.iter()
        .filter_map(|entry| {
            let kind = match entry.kind()? {
                ObjectType::Tree => FileKind::Directory,
                ObjectType::Blob => FileKind::File,
                _ => return None,
            };
            let name = entry.name()?.to_owned();

            Some(RepoFile { name, kind })
        })
        .collect()
}

fn repo_tree_from_tree(name: &str, tree: &Tree<'_>) -> RepoTree {
    RepoTree {
        name: name.to_owned(),
        kind: TreeKind::Directory(repo_files_from_tree(tree)),
    }
}

fn repo_tree_from_blob(name: &str, blob: &Blob<'_>) -> RepoTree {
    let mime = mime_guess::from_path(name).first_or_text_plain();
    let binary = match mime.type_() {
        mime::AUDIO | mime::FONT | mime::IMAGE | mime::VIDEO => true,
        mime::APPLICATION => {
            matches!(mime.subtype(), mime::OCTET_STREAM | mime::PDF)
        }
        _ => false,
    };

    RepoTree {
        name: name.to_owned(),
        kind: if binary {
            TreeKind::Binary(blob.size())
        } else {
            match str::from_utf8(blob.content()) {
                Ok(content) => TreeKind::Text(content.to_owned()),
                Err(_) => TreeKind::Binary(blob.size()),
            }
        },
    }
}
