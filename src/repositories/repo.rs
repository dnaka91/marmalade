use std::{borrow::ToOwned, io::BufRead, str};

use anyhow::{Context, Result};
use camino::Utf8Path;
use futures_util::FutureExt;
use git2::{Blob, BranchType, ErrorCode, ObjectType, Repository, Tree};
use tokio::fs;
use tracing::instrument;

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

    #[instrument(skip_all)]
    pub async fn exists(&self) -> bool {
        let (file, git) = tokio::join!(
            fs::metadata(DIRS.repo_info_file(self.user, self.repo)).map(|m| m.is_ok()),
            fs::metadata(DIRS.repo_git_dir(self.user, self.repo)).map(|m| m.is_ok())
        );

        file && git
    }

    #[instrument(skip_all)]
    pub async fn visible(&self, auth_user: &str, repo_user: &str) -> Result<bool> {
        if auth_user == repo_user {
            return Ok(true);
        }

        Ok(!self.load_info().await?.private)
    }

    pub async fn create(&self, description: String, private: bool) -> Result<bool> {
        if self.exists().await {
            return Ok(false);
        }

        let data = serde_json::to_vec_pretty(&UserRepo {
            name: self.repo.to_owned(),
            description,
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

    #[instrument(skip_all)]
    pub async fn list_branches(&self) -> Result<Vec<String>> {
        if !self.exists().await {
            return Ok(vec!["master".to_owned()]);
        }

        let repo_git = DIRS.repo_git_dir(self.user, self.repo);
        let branches = tokio::task::spawn_blocking(move || -> Result<_> {
            let repo = Repository::open(repo_git).context("failed opening repo")?;
            let branches = repo
                .branches(Some(BranchType::Local))
                .context("failed listing branches")?;

            branches
                .into_iter()
                .map(|branch| {
                    let (branch, _) = branch.context("failed iterating branches")?;
                    Ok(branch
                        .name()
                        .context("failed getting branch name")?
                        .unwrap()
                        .to_owned())
                })
                .collect()
        })
        .await??;

        Ok(branches)
    }

    #[instrument(skip_all)]
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
                    let file = std::fs::File::open(repo_git.join("HEAD"))?;
                    let file = std::io::BufReader::new(file);

                    let head = file.lines().next().context("repo's HEAD file is empty")??;

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

    pub async fn set_branch(&self, branch: &str) -> Result<()> {
        if !self.exists().await {
            return Ok(());
        }

        let repo_git = DIRS.repo_git_dir(self.user, self.repo);
        let branch = branch.to_owned();

        tokio::task::spawn_blocking(move || -> Result<_> {
            let repo = Repository::open(&repo_git).context("failed opening repo")?;
            let branch = repo
                .find_branch(&branch, BranchType::Local)
                .context("failed finding branch")?
                .into_reference();

            repo.set_head(branch.name().unwrap())
                .context("failed setting head")?;

            Ok(())
        })
        .await??;

        Ok(())
    }

    #[instrument(skip_all)]
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
                entry.name().is_some_and(|name| {
                    name.eq_ignore_ascii_case("README.md") || name.eq_ignore_ascii_case("README")
                }) && entry.kind().is_some_and(|kind| kind == ObjectType::Blob)
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

    #[instrument(skip_all)]
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

    #[instrument(skip_all)]
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

    #[instrument(skip_all)]
    pub async fn load_info(&self) -> Result<UserRepo> {
        let data = fs::read(DIRS.repo_info_file(self.user, self.repo)).await?;
        serde_json::from_slice(&data).map_err(Into::into)
    }

    pub async fn save_info(&self, info: &UserRepo) -> Result<()> {
        let real_file = DIRS.repo_info_file(self.user, self.repo);
        let temp_file = DIRS.repo_info_temp_file(self.user, self.repo);

        let buf = serde_json::to_vec_pretty(info)?;
        fs::write(&temp_file, &buf).await?;
        fs::rename(temp_file, real_file).await?;

        Ok(())
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
    RepoTree {
        name: name.to_owned(),
        kind: if blob.is_binary() {
            TreeKind::Binary(blob.size())
        } else {
            match str::from_utf8(blob.content()) {
                Ok(content) => TreeKind::Text(content.to_owned()),
                Err(_) => TreeKind::Binary(blob.size()),
            }
        },
    }
}
