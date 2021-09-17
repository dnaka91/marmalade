use axum::{
    extract::{Form, Path},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use comrak::{
    plugins::syntect::SyntectAdapter, ComrakExtensionOptions, ComrakOptions, ComrakPlugins,
    ComrakRenderPlugins,
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
    #[serde(deserialize_with = "crate::de::repo_name")]
    pub repo: String,
}

pub async fn index(
    _user: User,
    Path(path): Path<BasePath>,
) -> Result<impl IntoResponse, StatusTemplate> {
    info!(?path.user,?path.repo, "got repo index request");

    let repo_repo = RepoRepository::new(&path.user, &path.repo);

    if repo_repo.exists().await {
        let files = {
            let mut files = repo_repo.get_file_list().await.unwrap();
            files.sort_by_key(|file| file.kind);
            files
        };

        let readme = repo_repo.get_readme().await.unwrap().map_or_else(
            || "No project readme available".to_owned(),
            |readme| {
                comrak::markdown_to_html_with_plugins(
                    &readme,
                    &ComrakOptions {
                        extension: ComrakExtensionOptions {
                            strikethrough: true,
                            tagfilter: true,
                            table: true,
                            autolink: true,
                            tasklist: true,
                            superscript: true,
                            header_ids: Some("user-content-".to_owned()),
                            footnotes: false,
                            description_lists: false,
                            front_matter_delimiter: None,
                        },
                        ..ComrakOptions::default()
                    },
                    &ComrakPlugins {
                        render: ComrakRenderPlugins {
                            codefence_syntax_highlighter: Some(&SyntectAdapter::new(
                                "base16-ocean.dark",
                            )),
                        },
                    },
                )
            },
        );

        Ok(HtmlTemplate(templates::repo::Index {
            user: path.user,
            repo: path.repo,
            files,
            readme,
        }))
    } else {
        Err(StatusTemplate(StatusCode::NOT_FOUND))
    }
}

pub async fn create(_user: User) -> impl IntoResponse {
    HtmlTemplate(templates::repo::Create { message: None })
}

#[derive(Deserialize)]
pub struct Create {
    name: String,
    #[serde(default, deserialize_with = "crate::de::form_bool")]
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
