//! Dioxus のビルドの入力 (index.html、public/ の静的アセット、tailwind.css) の配線を検査する (0037)。
//!
//! ファイルを読むため native でだけ動かす (wasm のテストは test_web.rs)。

#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::PathBuf;

/// クレートのルート。静的アセットはここからの相対パスで置く。
fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// index.html が favicon と manifest と Tailwind の出力を参照していることを検査する。
/// コメントにも同じ文字列が現れ得るため、コメントを除いてからタグを検査する。属性の順序や
/// 空白の違いで壊れないよう、属性の組で検査する。
/// apple-touch-icon は iOS を対象から落としたため参照しない (ADR-0017)。
#[test]
fn index_html_references_the_static_assets() {
    let html =
        fs::read_to_string(crate_dir().join("index.html")).expect("index.html must be readable");
    let html = strip_comments(&html);

    for (rel, href) in [
        ("rel=\"icon\"", "href=\"/favicon.png\""),
        ("rel=\"manifest\"", "href=\"/manifest.json\""),
        ("rel=\"stylesheet\"", "href=\"/tailwind.css\""),
    ] {
        let tag = html
            .split('<')
            .find(|tag| tag.contains(rel) && tag.contains(href))
            .unwrap_or_else(|| panic!("index.html must have a link tag with {rel} and {href}"));
        assert!(tag.starts_with("link"), "the tag must be a link tag: {tag}");
    }
    assert!(
        !html.contains("apple-touch-icon"),
        "index.html must not reference apple-touch-icon"
    );
}

/// HTML のコメント (`<!--` から `-->` まで) を除く。コメントの中の文字列を検査しないため。
fn strip_comments(html: &str) -> String {
    let mut stripped = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find("<!--") {
        stripped.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            None => return stripped,
        }
    }
    stripped.push_str(rest);
    stripped
}

/// Flutter の frontend/web/ から引き継いだ静的アセットが public/ にあることを検査する (ADR-0017)。
#[test]
fn public_dir_has_the_static_assets() {
    let public_dir = crate_dir().join("public");

    for path in [
        "favicon.png",
        "icons/Icon-192.png",
        "icons/Icon-512.png",
        "icons/Icon-maskable-192.png",
        "icons/Icon-maskable-512.png",
        "manifest.json",
    ] {
        assert!(
            public_dir.join(path).is_file(),
            "public/{path} must be a file"
        );
    }
}

/// manifest.json を JSON として読み、名前とテーマ色と icons の参照先を検査する (ADR-0017)。
/// 文字列の一致ではなく値の比較にすることで、整形の変更や壊れた JSON を検出する。
#[test]
fn manifest_has_the_name_and_theme_color() {
    let public_dir = crate_dir().join("public");
    let source = fs::read_to_string(public_dir.join("manifest.json"))
        .expect("public/manifest.json must be readable");
    let manifest: serde_json::Value =
        serde_json::from_str(&source).expect("public/manifest.json must be valid JSON");

    assert_eq!(manifest["name"], "brewbook", "manifest.json name");
    assert_eq!(
        manifest["short_name"], "brewbook",
        "manifest.json short_name"
    );
    assert_eq!(
        manifest["theme_color"], "#f4ede2",
        "manifest.json theme_color"
    );
    assert_eq!(
        manifest["background_color"], "#f4ede2",
        "manifest.json background_color"
    );

    let icons = manifest["icons"]
        .as_array()
        .expect("manifest.json icons must be an array");
    assert!(!icons.is_empty(), "manifest.json icons must not be empty");
    for icon in icons {
        let src = icon["src"].as_str().expect("icons[].src must be a string");
        assert!(
            public_dir.join(src).is_file(),
            "public/{src} referenced by manifest.json must be a file"
        );
    }
}

/// dx が Tailwind CLI を起動する条件 (tailwind.css がクレートのルートにあること) を検査する。
#[test]
fn tailwind_css_imports_tailwind() {
    let tailwind = fs::read_to_string(crate_dir().join("tailwind.css"))
        .expect("tailwind.css must be readable");

    assert!(
        tailwind.contains("@import \"tailwindcss\""),
        "tailwind.css must import tailwindcss"
    );
    assert!(
        tailwind.contains("@source"),
        "tailwind.css must declare the sources to scan"
    );
}
