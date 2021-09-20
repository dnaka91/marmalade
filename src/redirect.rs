use std::borrow::Cow;

use axum::response::Redirect;
use percent_encoding::NON_ALPHANUMERIC;

pub fn to_root() -> Redirect {
    Redirect::to("/".parse().unwrap())
}

pub fn to_login() -> Redirect {
    Redirect::to("/login".parse().unwrap())
}

pub fn to_register() -> Redirect {
    Redirect::to("/register".parse().unwrap())
}

pub fn to_repo_create() -> Redirect {
    Redirect::to("/repo/create".parse().unwrap())
}

pub fn to_repo_index(user: &str, repo: &str) -> Redirect {
    let user = Cow::from(percent_encoding::utf8_percent_encode(
        user,
        NON_ALPHANUMERIC,
    ));
    let repo = Cow::from(percent_encoding::utf8_percent_encode(
        repo,
        NON_ALPHANUMERIC,
    ));

    Redirect::to(format!("/{}/{}", user, repo).parse().unwrap())
}

pub fn to_user_index(user: &str) -> Redirect {
    let user = Cow::from(percent_encoding::utf8_percent_encode(
        user,
        NON_ALPHANUMERIC,
    ));

    Redirect::to(format!("/{}", user).parse().unwrap())
}

#[cfg(test)]
mod tests {
    use axum::{http::header::LOCATION, response::IntoResponse};

    use super::*;

    fn get_location(redirect: Redirect) -> String {
        redirect
            .into_response()
            .headers()
            .get(LOCATION)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
    }

    #[test]
    fn simple_valid() {
        assert_eq!("/", get_location(to_root()));
        assert_eq!("/login", get_location(to_login()));
        assert_eq!("/register", get_location(to_register()));
        assert_eq!("/repo/create", get_location(to_repo_create()));
    }

    #[test]
    fn parameterized_valid() {
        assert_eq!(
            "/hello/world",
            get_location(to_repo_index("hello", "world"))
        );
        assert_eq!("/hello", get_location(to_user_index("hello")));
    }

    #[test]
    fn exotic_valid() {
        assert_eq!(
            "/h%C3%A9%20llo%0A%F0%9F%91%8D/w%C3%B6%0D%20rld%F0%9F%98%80",
            get_location(to_repo_index(
                "h\u{e9} llo\n\u{1f44d}",
                "w\u{f6}\r rld\u{1f600}"
            ))
        );
        assert_eq!(
            "/h%C3%A9%20llo%0A%F0%9F%91%8D",
            get_location(to_user_index("h\u{e9} llo\n\u{1f44d}"))
        );
    }
}
