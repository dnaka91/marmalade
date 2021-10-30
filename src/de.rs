use std::fmt;

use serde::de::{self, Deserializer, Visitor};

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
}
