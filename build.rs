use std::{
    collections::hash_map::DefaultHasher,
    env,
    hash::Hasher,
    io::{BufReader, Cursor},
    path::{Path, PathBuf},
};

use indoc::formatdoc;
use proc_macro2::TokenStream;
use quote::quote;
use sass_rs::{Options, OutputStyle};
use syntect::{highlighting::ThemeSet, html::ClassStyle};

fn main() {
    println!("cargo:rerun-if-changed=assets/");

    let root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    let (webfonts, base_route) = render_webfonts(&root);
    let main_css = render_main_css(&root, &out, base_route);

    let syntax = quote! {
        use headers::ETag;
        use once_cell::sync::Lazy;

        #main_css
        #webfonts
    };

    std::fs::write(out.join("assets.rs"), syntax.to_string()).unwrap();
}

fn render_main_css(root: &str, out: &Path, webfonts_route: String) -> TokenStream {
    let highlight = syntect::html::css_for_theme_with_class_style(
        &ThemeSet::load_from_reader(&mut BufReader::new(Cursor::new(
            &include_bytes!("assets/OneHalfLight.tmTheme")[..],
        )))
        .unwrap(),
        ClassStyle::SpacedPrefixed {
            prefix: "highlight-",
        },
    );

    let sass = formatdoc! {r#"
        @charset "utf-8";

        $fa-font-path: "{}";
        @import "{}/assets/main.sass";

        /*!
         * Syntect themes
         */
        {}
        "#,
        webfonts_route, root, highlight
    };

    let css = sass_rs::compile_string(
        &sass,
        Options {
            output_style: OutputStyle::Compressed,
            precision: 5,
            indented_syntax: false,
            include_paths: vec![root.to_owned()],
        },
    )
    .unwrap();

    let hash = {
        let mut hasher = DefaultHasher::new();
        hasher.write(css.as_bytes());
        hasher.finish()
    };
    let route = format!("/main-{:016x}.css", hash);
    let etag = format!("\"W/{:016x}\"", hash);

    std::fs::write(out.join("main.css"), css).unwrap();

    quote! {
        pub const MAIN_CSS_CONTENT: &str = include_str!(concat!(env!("OUT_DIR"), "/main.css"));
        pub const MAIN_CSS_ROUTE: &str = #route;
        pub const MAIN_CSS_HASH: Lazy<ETag> =Lazy::new(|| #etag.parse().unwrap());
    }
}

fn render_webfonts(root: &str) -> (TokenStream, String) {
    let entries = {
        let mut entries = std::fs::read_dir(format!("{}/assets/fontawesome/webfonts", root))
            .unwrap()
            .map(|entry| entry.unwrap())
            .collect::<Vec<_>>();
        entries.sort_by_key(|e| e.file_name());
        entries
    };

    let mut contents = Vec::new();
    let mut names = Vec::new();
    let mut hashes = Vec::new();
    let mut folder_hash = DefaultHasher::new();

    for entry in entries {
        let content = std::fs::read(entry.path()).unwrap();
        let hash = {
            let mut hasher = DefaultHasher::new();
            hasher.write(&content);
            hasher.finish()
        };
        let path = entry.path().into_os_string().into_string().unwrap();
        let name = format!("/{}", entry.file_name().into_string().unwrap());
        let etag = format!("\"W/{:016x}\"", hash);

        contents.push(quote! { include_bytes!(#path) });
        names.push(name);
        hashes.push(etag);
        folder_hash.write(&content);
    }

    let base_route = format!("/webfonts-{:016x}", folder_hash.finish());

    let syntax = quote! {
        pub const WEBFONTS_ROUTE: &str = #base_route;
        pub const WEBFONTS_CONTENT: &[&[u8]] = &[#(#contents),*];
        pub const WEBFONTS_NAME: &[&str] = &[#(#names),*];
        pub const WEBFONTS_HASH: Lazy<Vec<ETag>> = Lazy::new(|| {
            [#(#hashes),*].iter().map(|&hash| hash.parse().unwrap()).collect()
        });
    };

    (syntax, base_route)
}
