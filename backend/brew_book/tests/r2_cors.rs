//! R2 バケットの CORS の設定 (`cors.json`) の検査。
//!
//! 写真はサーバーを経由せずクライアントから R2 へ直接 PUT するため、R2 の S3 互換エンドポイントは
//! アプリとは別オリジンになる。R2 バケットの CORS で、アプリのオリジンからの PUT だけを許可する
//! (ADR-0003、ADR-0005、PRD のセキュリティ)。この設定は `mise run r2-setup` で R2 に適用する。
//!
//! 本番の workers.dev のサブドメインはアカウントごとに異なるため、`wrangler.toml` の `ORIGIN` と
//! 同じプレースホルダを置き、デプロイの前に実際の値へ置き換える (README のデプロイ手順)。

use serde_json::Value;

/// クレートのルートからの `cors.json` のパス。
const CORS_JSON: &str = "cors.json";

/// `cors.json` を読む。
fn cors() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(CORS_JSON);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("{} must be JSON: {error}", path.display()))
}

/// `cors.json` の唯一のルールの `allowed` を返す。
fn allowed() -> Value {
    let rules = cors()["rules"]
        .as_array()
        .unwrap_or_else(|| panic!("cors.json must have the rules array"))
        .clone();
    assert_eq!(rules.len(), 1, "cors.json must have exactly one rule");
    rules[0]["allowed"].clone()
}

/// `wrangler.toml` の `ORIGIN` の値 (本番のアプリのオリジン)。
fn configured_origin() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("wrangler.toml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    text.lines()
        .find_map(|line| {
            let line = line.trim();
            line.strip_prefix("ORIGIN")?
                .trim_start()
                .strip_prefix('=')?
                .trim()
                .strip_prefix('"')?
                .strip_suffix('"')
                .map(str::to_owned)
        })
        .unwrap_or_else(|| panic!("wrangler.toml must set the ORIGIN var"))
}

/// 文字列の配列を返す。
fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("the field must be an array: {value}"))
        .iter()
        .map(|item| {
            item.as_str()
                .unwrap_or_else(|| panic!("the item must be a string: {item}"))
                .to_owned()
        })
        .collect()
}

#[test]
fn cors_allows_put_from_the_app_origins() {
    let allowed = allowed();
    let origins = strings(&allowed["origins"]);
    // ローカルの開発 (http://localhost:8787) のオリジンを含む。
    assert!(
        origins.contains(&coffee_log::auth::DEFAULT_ORIGIN.to_owned()),
        "the origins must contain the local development origin: {origins:?}"
    );
    // 本番のオリジン (wrangler.toml の ORIGIN。デプロイの前に実際の値へ置き換える) を含む。
    let production = configured_origin();
    assert!(
        origins.contains(&production),
        "the origins must contain the production origin {production}: {origins:?}"
    );
    // ワイルドカードは許可しない (アプリのオリジンだけを許可する)。
    assert!(
        !origins.iter().any(|origin| origin.contains('*')),
        "the origins must not contain a wildcard: {origins:?}"
    );
    // 許可するのは PUT と Content-Type だけとする。
    assert_eq!(strings(&allowed["methods"]), vec!["PUT".to_owned()]);
    assert_eq!(
        strings(&allowed["headers"]),
        vec!["Content-Type".to_owned()]
    );
}
