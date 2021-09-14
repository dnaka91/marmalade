use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use directories_next::ProjectDirs;
use once_cell::sync::Lazy;

// Unwrap: We can't run the server without known where to place files, so panic here as there is no
// good recovery case other than throwing an error and shutting down.
pub static DIRS: Lazy<Utf8ProjectDirs> = Lazy::new(|| Utf8ProjectDirs::new().unwrap());

pub struct Utf8ProjectDirs {
    data_dir: Utf8PathBuf,
}

impl Utf8ProjectDirs {
    fn new() -> Result<Self> {
        let dirs = ProjectDirs::from("rocks", "dnaka91", env!("CARGO_PKG_NAME"))
            .context("failed finding project dirs")?;

        let data_dir = Utf8Path::from_path(dirs.data_dir())
            .context("project data dir is not valid UTF-8")?
            .to_owned();

        Ok(Self { data_dir })
    }

    #[inline]
    pub fn data_dir(&self) -> &Utf8Path {
        &self.data_dir
    }
}
