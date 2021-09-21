use askama::Template;

#[derive(Template)]
#[template(path = "user/index.html")]
pub struct Index {
    pub auth_user: Option<String>,
    pub user: String,
    pub repos: Vec<String>,
}

impl Index {
    fn auth_same_user(&self) -> bool {
        self.auth_user
            .as_deref()
            .map(|u| u == self.user)
            .unwrap_or_default()
    }
}

#[derive(Template)]
#[template(path = "user/list.html")]
pub struct List {
    pub auth_user: Option<String>,
    pub users: Vec<String>,
}
