use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index;

#[derive(Template)]
#[template(path = "login.html")]
pub struct Login;

#[derive(Template)]
#[template(path = "show.html")]
pub struct Show {
    pub username: Option<String>,
}
