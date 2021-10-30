use std::{io::ErrorKind, sync::Arc};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::dirs::DIRS;

pub type GlobalSettings = Arc<Settings>;

#[derive(Serialize, Deserialize)]
pub struct Settings {
    #[serde(with = "crate::ser::hex")]
    pub key: [u8; 64],
    pub tor: Option<Tor>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            key: crate::cookies::generate_key(),
            tor: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Tor {
    pub onion: String,
}

pub async fn load() -> Result<GlobalSettings> {
    let buf = match fs::read(DIRS.settings_file()).await {
        Ok(buf) => buf,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            let settings = Settings::default();
            save(&settings)
                .await
                .context("failed saving default settings")?;
            return Ok(Arc::new(settings));
        }
        Err(e) => return Err(e).context("failed loading settings"),
    };

    Ok(Arc::new(
        serde_json::from_slice(&buf).context("failed parsing settings")?,
    ))
}

async fn save(settings: &Settings) -> Result<()> {
    let real_file = DIRS.settings_file();
    let temp_file = DIRS.settings_temp_file();

    let buf = serde_json::to_vec_pretty(settings)?;
    fs::write(&temp_file, &buf).await?;
    fs::rename(temp_file, real_file).await?;

    Ok(())
}
