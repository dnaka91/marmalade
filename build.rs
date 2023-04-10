use std::{
    collections::hash_map::DefaultHasher,
    env, fs,
    hash::Hasher,
    io::{BufReader, Cursor},
    path::{Path, PathBuf},
};

use proc_macro2::TokenStream;
use quote::quote;
use syntect::{highlighting::ThemeSet, html::ClassStyle};

fn main() {
    println!("cargo:rerun-if-changed=assets/");

    let root = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    let (webfonts, base_route) = render_webfonts(&root);
    let main_css = render_main_css(&root, &out, base_route);
    let favicons = render_favicons(&root);

    let syntax = quote! {
        use headers::ETag;
        use once_cell::sync::Lazy;

        #main_css
        #webfonts
        #favicons
    };

    fs::write(out.join("assets.rs"), syntax.to_string()).unwrap();
}

fn render_main_css(root: &str, out: &Path, webfonts_route: String) -> TokenStream {
    let highlight = syntect::html::css_for_theme_with_class_style(
        &ThemeSet::load_from_reader(&mut BufReader::new(Cursor::new(
            &include_bytes!("assets/OneHalfDark.tmTheme")[..],
        )))
        .unwrap(),
        ClassStyle::SpacedPrefixed {
            prefix: "highlight-",
        },
    )
    .unwrap();

    let scss = format!(
        r#"
        @charset "utf-8";
        $fa-font-path: "{webfonts_route}";
        $fc-font-path: "{webfonts_route}";
        @import "assets/main.sass";
        "#
    );

    let css = grass::from_string(
        scss,
        &grass::Options::default()
            .input_syntax(grass::InputSyntax::Scss)
            .style(grass::OutputStyle::Compressed)
            .load_path(root),
    )
    .unwrap();
    let css = grass::from_string(
        format!("{css}\n{highlight}"),
        &grass::Options::default()
            .input_syntax(grass::InputSyntax::Css)
            .style(grass::OutputStyle::Compressed),
    )
    .unwrap();

    let hash = {
        let mut hasher = DefaultHasher::new();
        hasher.write(css.as_bytes());
        hasher.finish()
    };
    let route = format!("/main-{hash:016x}.css");
    let etag = format!("W/\"{hash:016x}\"");

    fs::write(out.join("main.css"), css).unwrap();

    quote! {
        pub const MAIN_CSS_ROUTE: &str = #route;
        pub static MAIN_CSS_CONTENT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/main.css"));
        pub static MAIN_CSS_HASH: Lazy<ETag> = Lazy::new(|| #etag.parse().unwrap());
    }
}

fn render_webfonts(root: &str) -> (TokenStream, String) {
    let entries = {
        let mut entries = fs::read_dir(format!("{root}/assets/fontawesome/webfonts"))
            .unwrap()
            .chain(fs::read_dir(format!("{root}/assets/firacode/woff2")).unwrap())
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
        let content = fs::read(entry.path()).unwrap();
        let hash = {
            let mut hasher = DefaultHasher::new();
            hasher.write(&content);
            hasher.finish()
        };
        let path = entry.path().into_os_string().into_string().unwrap();
        let name = entry.file_name().into_string().unwrap();
        let etag = format!("W/\"{hash:016x}\"");

        contents.push(quote! { include_bytes!(#path) });
        names.push(name);
        hashes.push(etag);
        folder_hash.write(&content);
    }

    let base_route = format!("/webfonts-{:016x}", folder_hash.finish());
    let route = format!("{base_route}/*path");

    let syntax = quote! {
        pub const WEBFONTS_ROUTE: &str = #route;
        pub static WEBFONTS_CONTENT: &[&[u8]] = &[#(#contents),*];
        pub static WEBFONTS_NAME: &[&str] = &[#(#names),*];
        pub static WEBFONTS_HASH: Lazy<Vec<ETag>> = Lazy::new(|| {
            [#(#hashes),*].iter().map(|&hash| hash.parse().unwrap()).collect()
        });
    };

    (syntax, base_route)
}

fn render_favicons(root: &str) -> TokenStream {
    let path = format!("{root}/assets/favicon.svg");
    let content = fs::read(&path).unwrap();
    let hash = {
        let mut hasher = DefaultHasher::new();
        hasher.write(&content);
        hasher.finish()
    };
    let route = format!("/favicon-{hash:016x}.svg");
    let etag = format!("W/\"{hash:016x}\"");

    quote! {
        pub const FAVICON_SVG_ROUTE: &str = #route;
        pub static FAVICON_SVG_CONTENT: &[u8] = include_bytes!(#path);
        pub static FAVICON_SVG_HASH: Lazy<ETag> = Lazy::new(|| #etag.parse().unwrap());
    }
}
