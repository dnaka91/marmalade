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

pub fn form_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(FormBoolVisitor)
}

struct FormBoolVisitor;

impl<'de> Visitor<'de> for FormBoolVisitor {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("boolean value encoded as `on` string for `true` and missing for `false`")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v == "on" {
            Ok(true)
        } else {
            Err(E::custom(format!("unknown boolean value `{}`", v)))
        }
    }
}

pub fn repo_name<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_string(RepoNameVisitor)
}

struct RepoNameVisitor;

impl<'de> Visitor<'de> for RepoNameVisitor {
    type Value = String;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("repository name with optional `.git` suffix")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(v.strip_suffix(".git").unwrap_or(v).to_owned())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(match v.strip_suffix(".git") {
            Some(stripped) => stripped.to_owned(),
            None => v,
        })
    }
}
