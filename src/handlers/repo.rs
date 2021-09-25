use axum::{
    extract::{Form, Path, Query},
    http::{StatusCode, Uri},
    response::IntoResponse,
};
use camino::{Utf8Path, Utf8PathBuf};
use once_cell::sync::Lazy;
use pulldown_cmark::{CodeBlockKind, Event, Tag};
use serde::Deserialize;
use syntect::{
    html::{ClassStyle, ClassedHTMLGenerator},
    parsing::{SyntaxDefinition, SyntaxReference, SyntaxSet},
    util::LinesWithEndings,
};
use tracing::info;

use crate::{
    cookies::{Cookie, Cookies},
    extract::User,
    models::TreeKind,
    redirect,
    repositories::{RepoRepository, UserRepository},
    response::{HtmlTemplate, SetCookies, StatusTemplate},
    session::COOKIE_ERROR,
    templates, validate,
};

#[derive(Deserialize)]
pub struct BasePath {
    #[serde(deserialize_with = "crate::de::percent")]
    pub user: String,
    #[serde(deserialize_with = "crate::de::repo_name")]
    pub repo: String,
}

pub async fn index(
    user: User,
    Path(path): Path<BasePath>,
) -> Result<impl IntoResponse, StatusTemplate> {
    info!(?path.user, ?path.repo, "got repo index request");

    let repo_repo = RepoRepository::for_repo(&path.user, &path.repo);

    if repo_repo.exists().await && repo_repo.visible(&user.username, &path.user).await.unwrap() {
        let files = {
            let mut files = repo_repo.get_file_list().await.unwrap();
            files.sort_by_key(|file| file.kind);
            files
        };

        let branch = repo_repo.get_branch().await.unwrap();
        let readme = repo_repo.get_readme().await.unwrap().map_or_else(
            || "No project readme available".to_owned(),
            |readme| render_markdown(&readme),
        );

        Ok(HtmlTemplate(templates::repo::Index {
            auth_user: Some(user.username),
            user: path.user,
            repo: path.repo,
            branch,
            files,
            readme,
        }))
    } else {
        Err(StatusTemplate(StatusCode::NOT_FOUND))
    }
}

#[derive(Deserialize)]
pub struct Tree {
    #[serde(deserialize_with = "crate::de::percent")]
    pub user: String,
    #[serde(deserialize_with = "crate::de::repo_name")]
    pub repo: String,
}

#[derive(Deserialize)]
pub struct TreeQuery {
    pub branch: String,
}

pub async fn tree(
    user: User,
    uri: Uri,
    Path(tree): Path<Tree>,
    Query(query): Query<TreeQuery>,
) -> Result<impl IntoResponse, StatusTemplate> {
    info!(?tree.user, ?tree.repo, ?query.branch, ?uri, "got repo tree request");

    let repo_repo = RepoRepository::for_repo(&tree.user, &tree.repo);

    if repo_repo.exists().await && repo_repo.visible(&user.username, &tree.user).await.unwrap() {
        let branches = repo_repo.list_branches().await.unwrap();
        let repo_tree = {
            let path = (uri.path() != "/").then(|| Utf8Path::new(&uri.path()[1..]));
            let tree = repo_repo.get_tree_list(&query.branch, path).await.unwrap();
            let mut tree = tree.ok_or(StatusTemplate(StatusCode::NOT_FOUND))?;

            match &mut tree.kind {
                TreeKind::Directory(files) => {
                    files.sort_by_key(|file| file.kind);
                }
                TreeKind::Text(text) => {
                    if let Some(ext) = Utf8Path::new(&tree.name).extension() {
                        let syntax = SYNTAX_SET
                            .find_syntax_by_extension(ext)
                            .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

                        *text = highlight_code(text, syntax);
                    }
                }
                TreeKind::Binary(_) => {}
            }

            tree
        };

        Ok(HtmlTemplate(templates::repo::Tree {
            auth_user: Some(user.username),
            user: tree.user,
            repo: tree.repo,
            branch: query.branch,
            branches,
            path: Utf8PathBuf::from(uri.path()[1..].to_owned()),
            tree: repo_tree,
        }))
    } else {
        Err(StatusTemplate(StatusCode::NOT_FOUND))
    }
}

pub async fn create(_user: User, mut cookies: Cookies) -> impl IntoResponse {
    let error = cookies
        .get(COOKIE_ERROR)
        .and_then(|cookie| cookie.value().parse().ok());

    if error.is_some() {
        cookies.remove(COOKIE_ERROR);
    }

    HtmlTemplate(templates::repo::Create { error })
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

    if !validate::repository(&create.name) {
        cookies.add(Cookie::new(
            COOKIE_ERROR,
            templates::repo::RepoCreateError::InvalidName.as_ref(),
        ));
        return SetCookies::new(redirect::to_repo_create(), cookies);
    }

    let user_repo = UserRepository::for_user(&user.username);
    let created = user_repo
        .repo(&create.name)
        .create(create.private)
        .await
        .unwrap();

    if created {
        SetCookies::new(
            redirect::to_repo_index(&user.username, &create.name),
            cookies,
        )
    } else {
        cookies.add(Cookie::new(
            COOKIE_ERROR,
            templates::repo::RepoCreateError::AlreadyExists.as_ref(),
        ));
        SetCookies::new(redirect::to_repo_create(), cookies)
    }
}

pub async fn delete(
    user: User,
    Path(path): Path<BasePath>,
) -> Result<impl IntoResponse, StatusTemplate> {
    info!(?path.user, ?path.repo, "got repo delete request");

    let repo_repo = RepoRepository::for_repo(&path.user, &path.repo);

    if user.username != path.user || !repo_repo.exists().await {
        return Err(StatusTemplate(StatusCode::NOT_FOUND));
    }

    Ok(HtmlTemplate(templates::repo::Delete { repo: path.repo }))
}

pub async fn delete_post(
    user: User,
    Path(path): Path<BasePath>,
) -> Result<impl IntoResponse, StatusTemplate> {
    info!(?path.user, ?path.repo, "got repo delete request");

    let repo_repo = RepoRepository::for_repo(&path.user, &path.repo);

    if user.username != path.user || !repo_repo.exists().await {
        return Err(StatusTemplate(StatusCode::NOT_FOUND));
    }

    let _deleted = repo_repo.delete().await.unwrap();

    Ok(redirect::to_user_index(&path.user))
}

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(|| {
    let mut builder = SyntaxSet::load_defaults_newlines().into_builder();
    builder.add(
        SyntaxDefinition::load_from_str(
            include_str!("../../assets/TOML.sublime-syntax"),
            true,
            Some("TOML"),
        )
        .unwrap(),
    );
    builder.build()
});

#[allow(clippy::option_if_let_else)]
fn render_markdown(text: &str) -> String {
    let default_syntax = SYNTAX_SET.find_syntax_plain_text();
    let mut syntax = None;

    let parser =
        pulldown_cmark::Parser::new_ext(text, pulldown_cmark::Options::all()).map(|event| {
            match event {
                Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
                    syntax = Some(
                        SYNTAX_SET
                            .find_syntax_by_token(&lang)
                            .unwrap_or(default_syntax),
                    );
                    Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang)))
                }
                Event::Text(text) => {
                    if let Some(syntax) = syntax {
                        Event::Html(highlight_code(&text, syntax).into())
                    } else {
                        Event::Text(text)
                    }
                }
                Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
                    syntax = None;
                    Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(lang)))
                }
                Event::Html(html) => Event::Text(html),
                event => event,
            }
        });

    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);

    html
}

fn highlight_code(text: &str, syntax: &SyntaxReference) -> String {
    let mut gen = ClassedHTMLGenerator::new_with_class_style(
        syntax,
        &*SYNTAX_SET,
        ClassStyle::SpacedPrefixed {
            prefix: "highlight-",
        },
    );

    for line in LinesWithEndings::from(text) {
        gen.parse_html_for_line_which_includes_newline(line);
    }

    gen.finalize()
}
