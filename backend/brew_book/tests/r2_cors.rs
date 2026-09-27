//! R2 バケットの CORS の設定 (`cors.json` と `cors.staging.json`) の検査。
//!
//! 写真はサーバーを経由せずクライアントから R2 へ直接 PUT するため、R2 の S3 互換エンドポイントは
//! アプリとは別オリジンになる。R2 バケットの CORS で、アプリのオリジンからの PUT だけを許可する
//! (ADR-0003、ADR-0005、PRD のセキュリティ)。
//!
//! 環境ごとにバケットを分けるため、CORS も環境ごとに持つ (ADR-0015)。
//! - `cors.json`: 本番のバケット。本番のオリジンだけを許可する (ローカルの開発は本番のバケットを使わない)。
//! - `cors.staging.json`: 検証用のバケット。ローカルの開発のオリジンと検証用のオリジンを許可する。
//!
//! 適用は `mise run r2-setup-production` と `mise run r2-setup-staging` で行う。

use serde_json::Value;

/// 本番のバケットの CORS の設定。
const CORS_PRODUCTION_JSON: &str = "cors.json";
/// 検証用のバケットの CORS の設定。
const CORS_STAGING_JSON: &str = "cors.staging.json";

/// CORS の設定を読む。
fn cors(file: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(file);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("{} must be JSON: {error}", path.display()))
}

/// CORS の設定の唯一のルールの `allowed` を返す。
fn allowed(file: &str) -> Value {
    let rules = cors(file)["rules"]
        .as_array()
        .unwrap_or_else(|| panic!("{file} must have the rules array"))
        .clone();
    assert_eq!(rules.len(), 1, "{file} must have exactly one rule");
    rules[0]["allowed"].clone()
}

/// `wrangler.toml` の指定した vars の節の `ORIGIN` の値を返す。
///
/// 節は `[env.production.vars]` や `[env.staging.vars]` の形で渡す。vars は環境ごとに定義し、
/// トップレベルからは継承されない (ADR-0015)。
fn configured_origin(section: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("wrangler.toml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let mut in_section = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_section = line == section;
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some(value) = line
            .strip_prefix("ORIGIN")
            .and_then(|rest| rest.trim_start().strip_prefix('='))
            .and_then(|rest| rest.trim().strip_prefix('"'))
            .and_then(|rest| rest.strip_suffix('"'))
        {
            return value.to_owned();
        }
    }
    panic!("wrangler.toml must set the ORIGIN var in {section}");
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

/// PUT と Content-Type だけを許可する (どちらの環境の設定も同じ)。
fn assert_put_and_content_type_only(file: &str) {
    let allowed = allowed(file);
    assert_eq!(strings(&allowed["methods"]), vec!["PUT".to_owned()]);
    assert_eq!(
        strings(&allowed["headers"]),
        vec!["Content-Type".to_owned()]
    );
    let origins = strings(&allowed["origins"]);
    assert!(
        !origins.iter().any(|origin| origin.contains('*')),
        "{file} must not contain a wildcard origin: {origins:?}"
    );
}

#[test]
fn production_cors_allows_only_the_production_origin() {
    let allowed = allowed(CORS_PRODUCTION_JSON);
    let origins = strings(&allowed["origins"]);
    let production = configured_origin("[env.production.vars]");
    // 本番のオリジンだけを許可する (ローカルの開発は検証用のバケットを使う)。
    assert_eq!(
        origins,
        vec![production],
        "the production CORS must allow only the production origin"
    );
    assert!(
        !origins.contains(&brew_book::auth::DEFAULT_ORIGIN.to_owned()),
        "the production CORS must not allow the local development origin"
    );
    assert_put_and_content_type_only(CORS_PRODUCTION_JSON);
}

#[test]
fn staging_cors_allows_the_local_and_staging_origins() {
    let allowed = allowed(CORS_STAGING_JSON);
    let origins = strings(&allowed["origins"]);
    // ローカルの開発 (http://localhost:8787) のオリジンを含む。
    assert!(
        origins.contains(&brew_book::auth::DEFAULT_ORIGIN.to_owned()),
        "the origins must contain the local development origin: {origins:?}"
    );
    // 検証用のオリジン (wrangler.toml の [env.staging.vars] の ORIGIN) を含む。
    let staging = configured_origin("[env.staging.vars]");
    assert!(
        origins.contains(&staging),
        "the origins must contain the staging origin {staging}: {origins:?}"
    );
    assert_put_and_content_type_only(CORS_STAGING_JSON);
}
