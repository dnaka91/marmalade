use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index {
    pub logged_in: bool,
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct Login;

#[derive(Template)]
#[template(path = "register.html")]
pub struct Register;

#[derive(Template)]
#[template(path = "show.html")]
pub struct Show {
    pub username: Option<String>,
}
