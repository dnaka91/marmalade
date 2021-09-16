use askama::Template;
use axum::http::StatusCode;

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index {
    pub logged_in: bool,
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct Login {
    pub message: Option<&'static str>,
}

#[derive(Template)]
#[template(path = "register.html")]
pub struct Register {
    pub message: Option<&'static str>,
}

#[derive(Template)]
#[template(path = "show.html")]
pub struct Show {
    pub username: Option<String>,
}

#[derive(Template)]
#[template(path = "error.html")]
pub struct Error {
    pub emoji: char,
    pub code: StatusCode,
    pub message: Option<&'static str>,
}

pub mod repo {
    use askama::Template;

    #[derive(Template)]
    #[template(path = "repo/index.html")]
    pub struct Index {
        pub name: String,
    }

    #[derive(Template)]
    #[template(path = "repo/create.html")]
    pub struct Create {
        pub message: Option<&'static str>,
    }
}
