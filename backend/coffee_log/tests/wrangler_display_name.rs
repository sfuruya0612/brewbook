//! 利用者向けの Worker の応答に表示名が含まれないことの結合テスト (FR-17)。
//!
//! 表示名を持つ利用者で利用者向けの全経路を呼び、どの応答の本文にも `display_name` と表示名の
//! 値が含まれないことを確認する。表示名は管理者が利用者を識別するためだけに使い、利用者向けの
//! Worker は応答に載せない。SQL が表示名を読まないことは単体テスト (`tests/test_queries.rs`) が
//! 検査する。
//!
//! テスト名の `wrangler_` は、`wrangler dev` を起動するテストを `backend:test` が名前で除外する
//! ための規約。サーバーは 1 つのテストファイルで 1 回だけ起動し、下ごしらえの SQL を先に実行する。

mod support;

use std::sync::OnceLock;

use coffee_log_core::routes::{Method, ROUTES};
use serde_json::json;
use support::http::ApiClient;
use support::seed::{user_id, Seed};
use support::ServerLease;

/// 下ごしらえする利用者の表示名。応答に現れてはならない。
const DISPLAY_NAME: &str = "秘密の表示名 <script>";

/// 下ごしらえに使う日時 (ISO 8601 UTC の固定長)。
const CREATED: &str = "2026-09-01T00:00:00.000Z";
const FUTURE: &str = "2099-01-01T00:00:00.000Z";

/// 記録の ID として使う UUID (下ごしらえしない ID は 404 になる)。
const RECORD_ID: &str = "00000000-0000-4000-8000-000000000009";

/// このテストファイルの下ごしらえと、テストが使う値。
struct TestData {
    /// 共有のサーバーの起動時に先に流す下ごしらえの SQL。
    seed_sql: String,
    /// 表示名を持つ利用者のセッション。
    session: String,
    /// その利用者のパスキーの ID (経路のパラメータに使う)。
    passkey: String,
}

/// 下ごしらえを 1 回だけ組み立てる。
fn data() -> &'static TestData {
    static DATA: OnceLock<TestData> = OnceLock::new();
    DATA.get_or_init(build_data)
}

/// 共有のサーバーを借りる。
fn server() -> ServerLease {
    support::shared_server("main", || {
        support::DevServer::start_with(|_| Vec::new(), &data().seed_sql)
    })
    .expect("wrangler dev must start")
}

/// 下ごしらえの SQL と、テストが使う値を作る。
fn build_data() -> TestData {
    let mut seed = Seed::new();
    let user = user_id(1);
    seed.user(&user, DISPLAY_NAME, CREATED);
    let session = seed.session(&user, FUTURE, CREATED);
    let passkey = seed
        .passkey(&user, "表示名の検査のパスキー", CREATED, None)
        .id;
    // 一覧とエクスポートが実際の行を返すように、記録を 1 つ入れる。
    seed.shop(&user, "表示名の検査の店", None, CREATED, CREATED, None);
    TestData {
        seed_sql: seed.sql(),
        session,
        passkey,
    }
}

/// 経路のパラメータを埋めて、呼ぶパスを作る。
fn concrete_path(pattern: &str) -> String {
    let id: &str = if pattern.starts_with("/api/passkeys/") {
        &data().passkey
    } else {
        RECORD_ID
    };
    pattern.replace(":field", "producer").replace(":id", id)
}

#[test]
fn wrangler_display_name_never_appears_in_the_responses() {
    let lease = server();
    lease.use_server(|server| {
        let data = data();
        let client = ApiClient::for_server(server, Some(&data.session));
        // セッションを消す経路 (ログアウトとアカウント削除) は最後に呼ぶ。先に呼ぶと、
        // 以降の経路が全て 401 になり、応答の中身を確かめられない。
        let mut routes: Vec<&coffee_log_core::routes::Route> = ROUTES.iter().collect();
        routes.sort_by_key(|route| matches!(route.name, "auth_logout" | "account_delete"));
        for route in routes {
            let path = concrete_path(route.pattern);
            let response = match route.method {
                Method::Get => client.get(&path),
                Method::Post => client.post_json(&path, &json!({})),
                Method::Patch => client.patch_json(&path, &json!({})),
                Method::Delete => client.delete(&path),
                Method::Put => continue,
            };
            let status = response.status().as_u16();
            let body = response.text().expect("the response body must be readable");
            assert!(
                !body.contains("display_name"),
                "{} {path} ({status}) must not contain display_name: {body}",
                route.method.as_str()
            );
            assert!(
                !body.contains(DISPLAY_NAME),
                "{} {path} ({status}) must not contain the display name: {body}",
                route.method.as_str()
            );
            // エクスポート (FR-14) は利用者の全記録を返すため、実際に読めていることを確かめる。
            if route.name == "export_get" {
                assert_eq!(status, 200, "the export must succeed: {body}");
                assert!(
                    body.contains("\"shops\""),
                    "the export must contain the tables: {body}"
                );
                assert!(
                    body.contains("表示名の検査の店"),
                    "the export must contain the seeded record: {body}"
                );
            }
        }
    });
}
