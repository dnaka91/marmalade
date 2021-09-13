use std::{fs, sync::Arc};

use anyhow::{Context, Result};
use serde::Deserialize;

pub type GlobalSettings = Arc<Settings>;

#[derive(Deserialize)]
pub struct Settings {
    #[serde(deserialize_with = "de::hex")]
    pub key: [u8; 64],
}

pub fn load() -> Result<GlobalSettings> {
    let locations = &[
        concat!("/etc/", env!("CARGO_PKG_NAME"), "/config.toml"),
        concat!("/app/", env!("CARGO_PKG_NAME"), ".toml"),
        concat!(env!("CARGO_PKG_NAME"), ".toml"),
    ];
    let buf = locations
        .iter()
        .find_map(|loc| fs::read(loc).ok())
        .context("failed finding settings")?;

    Ok(Arc::new(
        toml::from_slice(&buf).context("failed parsing settings")?,
    ))
}

mod de {
    use std::fmt;

    use serde::de::{self, Deserializer, Visitor};

    pub fn hex<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(HexVisitor)
    }

    struct HexVisitor;

    impl<'de> Visitor<'de> for HexVisitor {
        type Value = [u8; 64];

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a 64-byte array encoded as hex string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v.len() != 128 {
                return Err(E::custom("value must be exactly 128 characters long"));
            }

            let mut data = [0; 64];
            hex::decode_to_slice(v, &mut data).map_err(E::custom)?;

            Ok(data)
        }
    }
}
