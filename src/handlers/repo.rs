use axum::{
    extract::{Form, Path},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use tracing::info;

use crate::{
    cookies::{Cookie, Cookies},
    extract::User,
    repositories::{RepoRepository, UserRepository},
    response::{HtmlTemplate, SetCookies, StatusTemplate},
    session::COOKIE_ERROR,
    templates,
};

const CREATE_EMPTY_NAME: &str = "repo_create_empty_name";
const CREATE_EXISTS: &str = "repo_create_exists";

#[derive(Deserialize)]
pub struct BasePath {
    pub user: String,
    #[serde(deserialize_with = "de::repo_name")]
    pub repo: String,
}

pub async fn index(
    _user: User,
    Path(path): Path<BasePath>,
) -> Result<impl IntoResponse, StatusTemplate> {
    info!(?path.user,?path.repo, "got repo index request");

    if RepoRepository::new(&path.user, &path.repo).exists().await {
        Ok(HtmlTemplate(templates::repo::Index { name: path.repo }))
    } else {
        Err(StatusTemplate('🤷', StatusCode::NOT_FOUND))
    }
}

pub async fn create(_user: User) -> impl IntoResponse {
    HtmlTemplate(templates::repo::Create { message: None })
}

#[derive(Deserialize)]
pub struct Create {
    name: String,
    #[serde(default, deserialize_with = "de::form_bool")]
    private: bool,
}

pub async fn create_post(
    user: User,
    Form(create): Form<Create>,
    mut cookies: Cookies,
) -> impl IntoResponse {
    info!(?user.username, ?create.name, "got repo create request");

    if create.name.is_empty() {
        cookies.add(Cookie::new(COOKIE_ERROR, CREATE_EMPTY_NAME));
        return SetCookies::new(Redirect::to("/repo/create".parse().unwrap()), cookies);
    }

    let user_repo = UserRepository::for_user(&user.username);
    let created = user_repo
        .repo(&create.name)
        .create(create.private)
        .await
        .unwrap();

    if created {
        SetCookies::new(
            Redirect::to(
                format!("/{}/{}", user.username, create.name)
                    .parse()
                    .unwrap(),
            ),
            cookies,
        )
    } else {
        cookies.add(Cookie::new(COOKIE_ERROR, CREATE_EXISTS));
        SetCookies::new(Redirect::to("/repo/create".parse().unwrap()), cookies)
    }
}

mod de {
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
            formatter.write_str(
                "boolean value encoded as `on` string for `true` and missing for `false`",
            )
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
}
