use std::{io::ErrorKind, mem};

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use tokio::{fs, sync::RwLock};

use crate::{
    cookies,
    dirs::DIRS,
    models::{Archer, Settings, Tor, Tracing},
};

static STATE: Lazy<RwLock<Settings>> = Lazy::new(|| RwLock::new(Settings::default()));

pub struct SettingsRepository {
    _priv: (),
}

impl SettingsRepository {
    pub fn new() -> Self {
        Self { _priv: () }
    }

    pub async fn init() -> Result<()> {
        let settings = load().await?;
        *STATE.write().await = settings;

        Ok(())
    }

    pub async fn reset_key(&self) -> Result<()> {
        let mut settings = STATE.write().await;
        let old_key = mem::replace(&mut settings.key, cookies::generate_key());

        match save(&settings).await {
            Ok(()) => Ok(()),
            Err(e) => {
                settings.key = old_key;
                Err(e)
            }
        }
    }

    pub async fn get_key(&self) -> [u8; 64] {
        STATE.read().await.key
    }

    pub async fn get_tor_onion(&self) -> Option<String> {
        STATE.read().await.tor.as_ref().map(|t| t.onion.clone())
    }

    pub async fn get_tracing_archer(&self) -> Option<Archer> {
        STATE
            .read()
            .await
            .tracing
            .as_ref()
            .and_then(|t| t.archer.clone())
    }

    pub async fn set_tracing_archer(&self, archer: Option<Archer>) -> Result<()> {
        let mut settings = STATE.write().await;
        let tracing = settings.tracing.get_or_insert(Tracing::default());

        tracing.archer = archer;

        save(&settings).await
    }

    #[allow(clippy::option_if_let_else)]
    pub async fn set_tor_onion(&self, onion: String) -> Result<()> {
        let mut settings = STATE.write().await;
        let old_onion = {
            if onion.is_empty() {
                settings.tor = None;
                String::new()
            } else {
                let tor = settings.tor.get_or_insert(Tor::default());
                mem::replace(&mut tor.onion, onion)
            }
        };

        match save(&settings).await {
            Ok(()) => Ok(()),
            Err(e) => {
                if old_onion.is_empty() {
                    settings.tor = None;
                } else {
                    let tor = settings.tor.get_or_insert(Tor::default());
                    tor.onion = old_onion;
                }
                Err(e)
            }
        }
    }
}

pub async fn load() -> Result<Settings> {
    let buf = match fs::read(DIRS.settings_file()).await {
        Ok(buf) => buf,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            let settings = Settings {
                key: cookies::generate_key(),
                ..Settings::default()
            };

            save(&settings)
                .await
                .context("failed saving default settings")?;

            return Ok(settings);
        }
        Err(e) => return Err(e).context("failed loading settings"),
    };

    serde_json::from_slice(&buf).context("failed parsing settings")
}

async fn save(settings: &Settings) -> Result<()> {
    fs::create_dir_all(DIRS.data_dir()).await?;

    let real_file = DIRS.settings_file();
    let temp_file = DIRS.settings_temp_file();

    let buf = serde_json::to_vec_pretty(settings)?;

    fs::write(&temp_file, &buf).await?;
    fs::rename(temp_file, real_file).await?;

    Ok(())
}
