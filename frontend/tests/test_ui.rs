//! デザインシステムの静的な配線の単体テスト (0039)。
//!
//! ブラウザで計算済みスタイルを取るテストは `test_ui_web.rs` (frontend:test-web)。ここでは
//! ファイルを読むだけで確かめられることを検査する:
//!
//! - index.html が 3 つの書体とアイコンを Google Fonts から読むこと
//! - index.html が既定の Paper と `prefers-color-scheme` の Night を data-theme で決めること
//! - tailwind.css が原本 (docs/design/tokens.css) と部品の CSS を読み、ユーティリティを
//!   原本の変数に結び付けていること
//! - @theme に写した値 (角丸、書体、影) が原本と一致すること
//! - design.css が原本の部品の CSS (bundle.css) のクラスを全て持つこと
//! - アイコンの名前が Material Icons のリガチャの基底名であること

#![cfg(not(target_arch = "wasm32"))]

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

/// クレートのルート (frontend/)。
fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// リポジトリのルート。原本 (docs/design/) はここから読む。
fn repo_dir() -> PathBuf {
    crate_dir()
        .parent()
        .expect("the crate must be under the repository root")
        .to_path_buf()
}

/// ファイルを読む。
fn read(path: PathBuf) -> String {
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// index.html が 3 つの書体とアイコンを Google Fonts の CSS から読むことを検査する。
#[test]
fn index_html_loads_the_fonts_and_icons_from_google_fonts() {
    let html = read(crate_dir().join("index.html"));

    // 3 つの族は 1 つの CSS で読む (docs/design/README.md のタイポグラフィ)。
    let fonts = html
        .split('<')
        .find(|tag| {
            tag.contains("rel=\"stylesheet\"")
                && tag.contains("fonts.googleapis.com/css2")
                && tag.contains("IBM+Plex+Sans+JP")
                && tag.contains("IBM+Plex+Mono")
                && tag.contains("IBM+Plex+Serif")
        })
        .expect("index.html must load the three families from Google Fonts");
    assert!(fonts.starts_with("link"), "the tag must be a link: {fonts}");

    // アイコンは Material Icons Outlined (docs/design/README.md のアイコン)。
    let icons = html
        .split('<')
        .find(|tag| {
            tag.contains("rel=\"stylesheet\"")
                && tag.contains("fonts.googleapis.com/icon")
                && tag.contains("Material+Icons+Outlined")
        })
        .expect("index.html must load Material Icons Outlined from Google Fonts");
    assert!(icons.starts_with("link"), "the tag must be a link: {icons}");

    // 先に接続を開いて、最初の描画でフォントを待たない。
    assert!(
        html.contains("rel=\"preconnect\""),
        "index.html must preconnect to the font hosts"
    );
    assert!(
        html.contains("fonts.gstatic.com"),
        "index.html must preconnect to the font file host"
    );
}

/// 既定の Paper と、OS がダークのときの Night を data-theme で決めることを検査する。
#[test]
fn index_html_boots_the_theme_from_the_system() {
    let html = read(crate_dir().join("index.html"));

    assert!(
        html.contains("prefers-color-scheme: dark"),
        "index.html must follow prefers-color-scheme"
    );
    assert!(
        html.contains("data-theme"),
        "index.html must set data-theme before Dioxus starts"
    );
    assert!(
        html.contains("\"night\""),
        "index.html must switch to night when the system is dark"
    );
    assert!(
        html.contains("\"paper\""),
        "index.html must fall back to paper"
    );
}

/// tailwind.css が原本と部品の CSS を読み、ユーティリティを原本の変数に結び付けていることを
/// 検査する (bg-paper、text-ink、p-4、rounded-md など)。
#[test]
fn tailwind_css_maps_the_design_tokens() {
    let tailwind = read(crate_dir().join("tailwind.css"));

    assert!(
        tailwind.contains("@import \"tailwindcss\""),
        "tailwind.css must import tailwindcss"
    );
    assert!(
        tailwind.contains("../docs/design/tokens.css"),
        "tailwind.css must read the original tokens"
    );
    assert!(
        tailwind.contains("./src/ui/design.css"),
        "tailwind.css must read the component CSS"
    );
    for mapping in [
        "--color-paper: var(--paper)",
        "--color-ink: var(--ink)",
        "--color-roast: var(--roast)",
        "--color-crema: var(--crema)",
        "--color-signal: var(--signal)",
        "--spacing-1: var(--space-1)",
        "--spacing-4: var(--space-4)",
        "--spacing-12: var(--space-12)",
    ] {
        assert!(
            tailwind.contains(mapping),
            "tailwind.css must map {mapping}"
        );
    }
}

/// CSS のコメントを除く。
fn strip_comments(css: &str) -> String {
    let mut stripped = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        stripped.push_str(&rest[..start]);
        match rest[start..].find("*/") {
            Some(end) => rest = &rest[start + end + 2..],
            None => return stripped,
        }
    }
    stripped.push_str(rest);
    stripped
}

/// CSS の宣言 (`--名前: 値;`) の値を返す。最初の宣言を読む。
fn declaration(css: &str, name: &str) -> String {
    let needle = format!("{name}:");
    let start = css
        .find(&needle)
        .unwrap_or_else(|| panic!("{name} must be declared"));
    let rest = &css[start + needle.len()..];
    let end = rest
        .find(';')
        .unwrap_or_else(|| panic!("{name} must end with ;"));
    rest[..end].trim().to_string()
}

/// @theme に写した値 (角丸、書体、影) が原本 (docs/design/tokens.css) と一致することを検査する。
///
/// 色と余白は `var(--...)` で原本を参照するため写しが無い。角丸と書体と影は Tailwind の
/// 名前空間と同じ名前になり、@theme の中で自分自身を参照できないため値を写している。
#[test]
fn the_theme_values_in_tailwind_css_match_the_tokens() {
    let tailwind = read(crate_dir().join("tailwind.css"));
    let tokens = read(repo_dir().join("docs/design/tokens.css"));

    for name in [
        "--radius-sm",
        "--radius-md",
        "--radius-lg",
        "--radius-full",
        "--font-sans",
        "--font-mono",
        "--font-serif",
        "--shadow-float",
    ] {
        assert_eq!(
            declaration(&tailwind, name),
            declaration(&tokens, name),
            "{name} in tailwind.css must match docs/design/tokens.css"
        );
    }
}

/// CSS からクラス名を集める (コメントを除き、`.` の次の識別子を読む)。
fn class_names(css: &str) -> BTreeSet<String> {
    let css = strip_comments(css);
    let bytes = css.as_bytes();
    let mut names = BTreeSet::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'.'
            && index + 1 < bytes.len()
            && (bytes[index + 1].is_ascii_alphabetic()
                || bytes[index + 1] == b'_'
                || bytes[index + 1] == b'-')
        {
            let start = index + 1;
            let mut end = start;
            while end < bytes.len()
                && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_' || bytes[end] == b'-')
            {
                end += 1;
            }
            names.insert(css[start..end].to_string());
            index = end;
        } else {
            index += 1;
        }
    }
    names
}

/// design.css が原本の部品の CSS (bundle.css) のクラスを全て持つことを検査する。
///
/// プレビューの外枠 (`.stage`、`.tight`、`.phone`、`.screen`、`.wide`、`.stage-note`) だけは
/// 画面ではないため写さない。この検査で、原本との突き合わせが機械的にできる。
#[test]
fn the_component_css_covers_the_classes_of_the_bundle() {
    let bundle = read(repo_dir().join("docs/design/components/bundle.css"));
    let design = read(crate_dir().join("src/ui/design.css"));
    let bundle_classes = class_names(&bundle);
    let design_classes = class_names(&design);

    // プレビューの外枠 (bundle.css の「カードの並び」) は画面ではないので写さない。
    let preview_only = ["stage", "tight", "phone", "screen", "wide", "stage-note"];
    let missing: Vec<&String> = bundle_classes
        .iter()
        .filter(|name| !design_classes.contains(*name) && !preview_only.contains(&name.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "design.css must have every class of bundle.css (except the preview frame): {missing:?}"
    );
}

/// AppBar が印 (最上位の画面) を訳語の名前で出し、`wordmark` prop と `dioxus_router` を
/// 持たないことを検査する (0047、0054)。
#[test]
fn the_app_bar_has_the_mark_from_the_translations_without_the_router() {
    let source = read(crate_dir().join("src/ui/app_bar.rs"));

    assert!(
        source.contains("BrewbookMark"),
        "the AppBar must show the BrewbookMark"
    );
    assert!(
        source.contains("Key::AppTitle"),
        "the AppBar must name the mark from the translations"
    );
    assert!(
        !source.contains("wordmark"),
        "the AppBar must not have the wordmark prop"
    );
    assert!(
        !source.contains("dioxus_router"),
        "the AppBar must not depend on dioxus_router"
    );
}

/// アイコンの名前が Material Icons のリガチャの基底名になっていることを検査する (0039 の
/// レビューの指摘)。
///
/// Flutter の API 名 (`Icons.*_outlined`) を渡すとリガチャが解決せず、名前がそのまま文字と
/// して表示される。`src/ui` の `Icon` の `name` に `_outlined` が付いていないことを見る。
#[test]
fn the_icon_names_are_not_the_flutter_api_names() {
    let ui_dir = crate_dir().join("src/ui");
    let mut checked = 0;
    for entry in fs::read_dir(&ui_dir).expect("src/ui must be readable") {
        let path = entry.expect("the entry must be readable").path();
        if path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let source = read(path);
        for (index, _) in source.match_indices("name: \"") {
            let rest = &source[index + "name: \"".len()..];
            let end = rest.find('"').expect("the icon name must be closed");
            let name = &rest[..end];
            assert!(
                !name.ends_with("_outlined"),
                "the icon name must be a Material Icons ligature, not a Flutter API name: {name}"
            );
            checked += 1;
        }
    }
    assert!(
        checked > 0,
        "the check must find the icon names in src/ui (found {checked})"
    );
}
