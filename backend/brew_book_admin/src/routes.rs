//! 管理者 Worker の経路の台帳 (0018、ADR-0008)。
//!
//! 経路名はテストの識別子に使う。成功指標の照合は、この台帳を入力にした
//! データ駆動の結合テスト (`support::admin_suite_covers_ledger`) で行う。
//!
//! 認証は Worker 単位の Cloudflare Access が担うため、アプリ内の認証を要する経路は無い
//! (`auth_required` は全て false。ADR-0008)。

use brew_book_core::routes::{Method, OkTest, Route};

/// 管理者画面と管理者 API の経路。
///
/// `GET /` は利用者の一覧、`POST /users` は利用者の作成 (入力は表示名)、
/// `POST /users/:id/tokens` は登録用トークンの発行を担う。
/// 正常系はどれも CI で実行する (`ok_test` は `Ci`)。
pub const ROUTES: &[Route] = &[
    Route {
        name: "users_list",
        method: Method::Get,
        pattern: "/",
        auth_required: false,
        has_input: false,
        ok_test: OkTest::Ci,
    },
    Route {
        name: "users_create",
        method: Method::Post,
        pattern: "/users",
        auth_required: false,
        has_input: true,
        ok_test: OkTest::Ci,
    },
    Route {
        name: "tokens_create",
        method: Method::Post,
        pattern: "/users/:id/tokens",
        auth_required: false,
        // 経路のパラメータ (`:id`) は入力に数えない (0017 の `suggestions_list` と同じ扱い)。
        has_input: false,
        ok_test: OkTest::Ci,
    },
];
