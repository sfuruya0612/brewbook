//! 統計と評価の推移の API の結合テスト (HTTP)。
//!
//! 4 つの経路の応答の形、区間のキーの形式と昇順、記録の無い区間を返さないこと、通貨コードの
//! 昇順 (null が先頭)、条件が NULL の項目の null、評価の推移の昇順と NULL の除外を確認する。
//! 日と月の境界をまたぐ抽出日時を投入し、UTC オフセット +540 (日本標準時) と -300 で区切る
//! 区間が変わることと、期間の端が端末のローカル時刻の日付になることを確認する。
//! 他の利用者の記録とアーカイブ済みの記録が結果に含まれないこと、参照先の購入がアーカイブ
//! 済みでも抽出自身がアーカイブ済みでなければ統計に含まれることも確認する。
//!
//! テスト名の `wrangler_` は、`wrangler dev` を起動するテストを `backend:test` が名前で除外するための規約。
//! サーバーは 1 つのテストファイルで 1 回だけ起動し、下ごしらえの SQL を先に実行する。
//! テストは並行に走るため、記録を書き換えるテストは利用者ごとに分ける (このファイルのテストは
//! 下ごしらえを読むだけ)。

mod support;

use std::sync::OnceLock;

use reqwest::blocking::Response;
use serde_json::{json, Value};
use support::http::{error_code, read, ApiClient};
use support::seed::{user_id, Seed, SeededBrew, SeededPurchase};
use support::ServerLease;

/// 下ごしらえの行の作成日時。
const CREATED: &str = "2026-09-01T00:00:00.000Z";
/// セッションの有効期限 (十分に先の時刻)。
const FUTURE: &str = "2099-01-01T00:00:00.000Z";
/// アーカイブの日時。
const ARCHIVED: &str = "2026-09-22T00:00:00.000Z";

/// このテストファイルの下ごしらえと、テストが使う値。
struct TestData {
    seed_sql: String,
    /// 抽出の統計の利用者のセッション。日と月の境界をまたぐ抽出を持つ。
    stats_session: String,
    /// 同一の購入日の購入 A (評価の推移の主な検査に使う)。
    stats_purchase_a: SeededPurchase,
    /// 同一の購入日の購入 B。
    stats_purchase_b: SeededPurchase,
    /// アーカイブ済みの購入 (参照先がアーカイブ済みでも抽出を統計に含めることの検査に使う)。
    stats_archived_purchase: SeededPurchase,
    /// 評価を持つ抽出 (2026-09-20T15:00:00.000Z。+540 では 09-21、-300 では 09-20 の区間)。
    brew_a: SeededBrew,
    /// 評価が NULL の抽出 (2026-09-21T14:59:59.999Z。両方のオフセットで 09-21 の区間)。
    brew_b: SeededBrew,
    /// 評価を持つ抽出 (2026-09-21T15:00:00.000Z。+540 では 09-22、-300 では 09-21 の区間)。
    brew_c: SeededBrew,
    /// 評価を持つ抽出 (2026-09-30T14:59:59.999Z。月の境界の手前。湯量と温度と時間が NULL)。
    brew_d: SeededBrew,
    /// 評価を持つ抽出 (2026-09-20T14:59:59.999Z。豆の量が NULL)。
    brew_f: SeededBrew,
    /// 評価を持つ抽出 (2026-09-21T01:00:00.000Z。豆の量が NULL。購入がアーカイブ済み)。
    brew_i: SeededBrew,
    /// アーカイブ済みの抽出 (統計と評価の推移に含まれない)。
    brew_archived: SeededBrew,
    /// 購入金額と重量の統計の利用者のセッション。
    purchase_session: String,
    /// 2026-09-21 の 1200 JPY、重量 200。抽出を持たない。
    purchase_jpy: SeededPurchase,
    /// 他の利用者のセッション (404 と分離の検査に使う)。
    other_session: String,
    other_purchase: SeededPurchase,
    other_brew: SeededBrew,
    /// 記録が無い利用者のセッション。
    empty_session: String,
    /// 豆の量の合計の丸めの検査に使う利用者 (0.1 g と 0.2 g) のセッション。
    rounding_session: String,
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

    // 抽出の統計の利用者。
    let stats_user = user_id(1);
    seed.user(&stats_user, "stats user", CREATED);
    let stats_session = seed.session(&stats_user, FUTURE, CREATED);
    let stats_product = seed.product(&stats_user, "統計の豆", CREATED, CREATED, None);
    let stats_purchase_a = seed.purchase_with_numbers(
        &stats_user,
        &stats_product.id,
        None,
        "2026-09-15",
        Some(1200),
        Some("JPY"),
        Some(200),
        CREATED,
        CREATED,
        None,
    );
    let stats_purchase_b = seed.purchase_with_numbers(
        &stats_user,
        &stats_product.id,
        None,
        "2026-09-15",
        Some(800),
        Some("USD"),
        Some(150),
        CREATED,
        CREATED,
        None,
    );
    let stats_archived_purchase = seed.purchase_with_numbers(
        &stats_user,
        &stats_product.id,
        None,
        "2026-09-15",
        None,
        None,
        None,
        CREATED,
        ARCHIVED,
        Some(ARCHIVED),
    );
    // 日と月の境界をまたぐ抽出日時 (コメントの区間は +540 の端末でのローカル時刻)。
    let brew_a = seed.brew_with_numbers(
        &stats_user,
        &stats_purchase_a.id,
        "2026-09-20T15:00:00.000Z", // 09-21 の 00:00
        Some(12.0),
        Some(200.0),
        Some(92.5),
        Some(150),
        Some(5),
        CREATED,
        CREATED,
        None,
    );
    let brew_b = seed.brew_with_numbers(
        &stats_user,
        &stats_purchase_a.id,
        "2026-09-21T14:59:59.999Z", // 09-21 の 23:59:59.999
        Some(10.5),
        Some(180.0),
        Some(91.0),
        Some(140),
        None,
        CREATED,
        CREATED,
        None,
    );
    let brew_c = seed.brew_with_numbers(
        &stats_user,
        &stats_purchase_b.id,
        "2026-09-21T15:00:00.000Z", // 09-22 の 00:00
        Some(10.0),
        Some(150.0),
        Some(90.0),
        Some(120),
        Some(4),
        CREATED,
        CREATED,
        None,
    );
    let brew_d = seed.brew_with_numbers(
        &stats_user,
        &stats_purchase_b.id,
        "2026-09-30T14:59:59.999Z", // 09-30 の 23:59:59.999
        Some(7.5),
        None,
        None,
        None,
        Some(3),
        CREATED,
        CREATED,
        None,
    );
    seed.brew_with_numbers(
        &stats_user,
        &stats_purchase_b.id,
        "2026-09-30T15:00:00.000Z", // 10-01 の 00:00
        Some(8.0),
        Some(170.0),
        Some(92.0),
        Some(130),
        None,
        CREATED,
        CREATED,
        None,
    );
    let brew_f = seed.brew_with_numbers(
        &stats_user,
        &stats_purchase_a.id,
        "2026-09-20T14:59:59.999Z", // 09-20 の 23:59:59.999
        None,
        None,
        None,
        None,
        Some(2),
        CREATED,
        CREATED,
        None,
    );
    // 購入がアーカイブ済みでも、抽出自身がアーカイブ済みでなければ統計に含める (ADR-0006)。
    let brew_i = seed.brew_with_numbers(
        &stats_user,
        &stats_archived_purchase.id,
        "2026-09-21T01:00:00.000Z", // 09-21 の 10:00
        None,
        Some(160.0),
        Some(88.0),
        Some(100),
        Some(3),
        CREATED,
        CREATED,
        None,
    );
    let brew_archived = seed.brew_with_numbers(
        &stats_user,
        &stats_purchase_a.id,
        "2026-09-21T02:00:00.000Z",
        Some(100.0),
        Some(100.0),
        Some(99.0),
        Some(999),
        Some(1),
        CREATED,
        ARCHIVED,
        Some(ARCHIVED),
    );

    // 購入金額と重量の統計の利用者。同じ購入日と、離れた購入日を含める。
    let purchase_user = user_id(2);
    seed.user(&purchase_user, "purchase stats user", CREATED);
    let purchase_session = seed.session(&purchase_user, FUTURE, CREATED);
    let purchase_product = seed.product(&purchase_user, "購入統計の豆", CREATED, CREATED, None);
    let purchase_jpy = seed.purchase_with_numbers(
        &purchase_user,
        &purchase_product.id,
        None,
        "2026-09-21",
        Some(1200),
        Some("JPY"),
        Some(200),
        CREATED,
        CREATED,
        None,
    );
    seed.purchase_with_numbers(
        &purchase_user,
        &purchase_product.id,
        None,
        "2026-09-21",
        Some(800),
        Some("USD"),
        Some(150),
        CREATED,
        CREATED,
        None,
    );
    seed.purchase_with_numbers(
        &purchase_user,
        &purchase_product.id,
        None,
        "2026-09-21",
        None,
        None,
        None,
        CREATED,
        CREATED,
        None,
    );
    seed.purchase_with_numbers(
        &purchase_user,
        &purchase_product.id,
        None,
        "2026-09-22",
        Some(500),
        Some("JPY"),
        None,
        CREATED,
        CREATED,
        None,
    );
    seed.purchase_with_numbers(
        &purchase_user,
        &purchase_product.id,
        None,
        "2026-09-22",
        None,
        None,
        Some(100),
        CREATED,
        CREATED,
        None,
    );
    seed.purchase_with_numbers(
        &purchase_user,
        &purchase_product.id,
        None,
        "2026-09-20",
        Some(1000),
        Some("JPY"),
        Some(300),
        CREATED,
        CREATED,
        None,
    );
    seed.purchase_with_numbers(
        &purchase_user,
        &purchase_product.id,
        None,
        "2026-09-21",
        Some(9999),
        Some("JPY"),
        Some(999),
        CREATED,
        ARCHIVED,
        Some(ARCHIVED),
    );
    seed.purchase_with_numbers(
        &purchase_user,
        &purchase_product.id,
        None,
        "2026-10-01",
        Some(2000),
        Some("JPY"),
        Some(400),
        CREATED,
        CREATED,
        None,
    );

    // 他の利用者。購入と抽出を 1 つずつ持ち、統計と 404 の検査に使う。
    let other_user = user_id(3);
    seed.user(&other_user, "other user", CREATED);
    let other_session = seed.session(&other_user, FUTURE, CREATED);
    let other_product = seed.product(&other_user, "他人の豆", CREATED, CREATED, None);
    let other_purchase = seed.purchase_with_numbers(
        &other_user,
        &other_product.id,
        None,
        "2026-09-21",
        Some(5000),
        Some("JPY"),
        Some(500),
        CREATED,
        CREATED,
        None,
    );
    let other_brew = seed.brew_with_numbers(
        &other_user,
        &other_purchase.id,
        "2026-09-21T02:00:00.000Z",
        Some(200.0),
        Some(200.0),
        Some(99.0),
        Some(999),
        Some(1),
        CREATED,
        CREATED,
        None,
    );

    // 記録が無い利用者。
    let empty_user = user_id(4);
    seed.user(&empty_user, "empty user", CREATED);
    let empty_session = seed.session(&empty_user, FUTURE, CREATED);

    // 豆の量の合計の丸めの利用者 (0.1 + 0.2 は浮動小数点では 0.30000000000000004 になる)。
    let rounding_user = user_id(5);
    seed.user(&rounding_user, "rounding user", CREATED);
    let rounding_session = seed.session(&rounding_user, FUTURE, CREATED);
    let rounding_product = seed.product(&rounding_user, "丸めの豆", CREATED, CREATED, None);
    let rounding_purchase = seed.purchase_with_numbers(
        &rounding_user,
        &rounding_product.id,
        None,
        "2026-09-15",
        None,
        None,
        None,
        CREATED,
        CREATED,
        None,
    );
    for (index, dose) in [(1, 0.1), (2, 0.2)] {
        seed.brew_with_numbers(
            &rounding_user,
            &rounding_purchase.id,
            &format!("2026-09-15T0{index}:00:00.000Z"),
            Some(dose),
            None,
            None,
            None,
            None,
            CREATED,
            CREATED,
            None,
        );
    }

    TestData {
        seed_sql: seed.sql(),
        stats_session,
        stats_purchase_a,
        stats_purchase_b,
        stats_archived_purchase,
        brew_a,
        brew_b,
        brew_c,
        brew_d,
        brew_f,
        brew_i,
        brew_archived,
        purchase_session,
        purchase_jpy,
        other_session,
        other_purchase,
        other_brew,
        empty_session,
        rounding_session,
    }
}

/// セッションの Cookie を持たないクライアント。
fn anonymous(base_url: &str) -> ApiClient {
    ApiClient::new(base_url, None)
}

/// 応答の状態コードと本体を確かめる。
fn assert_status(response: Response, expected: u16) -> Value {
    let (status, body) = read(response);
    assert_eq!(status, expected, "the response body was {body}");
    body
}

/// 未認証の呼び出しが 401 になることを確かめる。
fn assert_unauthorized(response: Response) {
    let body = assert_status(response, 401);
    assert_eq!(error_code(&body), Some("unauthorized"));
}

/// 入力不正の呼び出しが 400 になることを確かめる。
fn assert_bad_request(response: Response) {
    let body = assert_status(response, 400);
    assert_eq!(error_code(&body), Some("bad_request"), "{body}");
}

/// 対象が無い呼び出しが 404 になることを確かめる。
fn assert_not_found(response: Response) {
    let body = assert_status(response, 404);
    assert_eq!(error_code(&body), Some("not_found"), "{body}");
}

/// 区間の応答 1 件 (抽出回数と豆の消費量) を確かめる。
fn assert_brew_period(entry: &Value, period: &str, brew_count: i64, dose_grams: f64) {
    assert_eq!(entry["period"], period, "{entry}");
    assert_eq!(entry["brew_count"], brew_count, "{entry}");
    assert_eq!(
        entry["dose_grams"].as_f64(),
        Some(dose_grams),
        "the dose must be rounded to one decimal place: {entry}"
    );
}

/// 区間と通貨コードの組の応答 1 件 (購入金額と重量) を確かめる。
fn assert_purchase_period(
    entry: &Value,
    period: &str,
    price_currency: Option<&str>,
    price_amount: i64,
    weight_grams: i64,
    purchase_count: i64,
) {
    assert_eq!(entry["period"], period, "{entry}");
    match price_currency {
        Some(currency) => assert_eq!(entry["price_currency"], currency, "{entry}"),
        None => assert_eq!(entry["price_currency"], Value::Null, "{entry}"),
    }
    assert_eq!(entry["price_amount"], price_amount, "{entry}");
    assert_eq!(entry["weight_grams"], weight_grams, "{entry}");
    assert_eq!(entry["purchase_count"], purchase_count, "{entry}");
}

/// 統計の応答の並びを確かめる。
fn assert_brews(body: &Value) -> Vec<Value> {
    body["brews"]
        .as_array()
        .unwrap_or_else(|| panic!("the response must have brews: {body}"))
        .clone()
}

/// 購入金額と重量の応答の並びを確かめる。
fn assert_purchases(body: &Value) -> Vec<Value> {
    body["purchases"]
        .as_array()
        .unwrap_or_else(|| panic!("the response must have purchases: {body}"))
        .clone()
}

mod stats_brews {
    //! `GET /api/stats/brews` のテスト (FR-18)。

    use super::*;

    #[test]
    fn wrangler_stats_brews_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.stats_session));

        // 1 日だけを指定する。豆の量が NULL の抽出も件数に数え、消費量には加えない (FR-18)。
        let body = assert_status(
            client.get("/api/stats/brews?start=2026-09-21&end=2026-09-21&granularity=day&utc_offset_minutes=540"),
            200,
        );
        let brews = assert_brews(&body);
        // A (12.0) と B (10.5、評価は NULL) と I (豆の量は NULL) が 09-21 の区間になる。
        // C は +540 では 09-22、F は 09-20 の区間になり、G (アーカイブ済み) は含まれない。
        assert_eq!(brews.len(), 1, "{body}");
        assert_brew_period(&brews[0], "2026-09-21", 3, 22.5);

        // 期間を省略すると全期間になる。記録の無い区間は返さず、区間はキーの昇順で並ぶ (FR-18)。
        let body = assert_status(
            client.get("/api/stats/brews?granularity=day&utc_offset_minutes=540"),
            200,
        );
        let brews = assert_brews(&body);
        assert_eq!(brews.len(), 5, "{body}");
        assert_brew_period(&brews[0], "2026-09-20", 1, 0.0);
        assert_brew_period(&brews[1], "2026-09-21", 3, 22.5);
        assert_brew_period(&brews[2], "2026-09-22", 1, 10.0);
        assert_brew_period(&brews[3], "2026-09-30", 1, 7.5);
        assert_brew_period(&brews[4], "2026-10-01", 1, 8.0);

        // 月別は区間のキーが `YYYY-MM` になる。月をまたぐ抽出日時は月の境界で分かれる (FR-18)。
        let body = assert_status(
            client.get("/api/stats/brews?granularity=month&utc_offset_minutes=540"),
            200,
        );
        let brews = assert_brews(&body);
        assert_eq!(brews.len(), 2, "{body}");
        assert_brew_period(&brews[0], "2026-09", 6, 40.0);
        assert_brew_period(&brews[1], "2026-10", 1, 8.0);

        // 月の境界をまたぐ期間を指定する。09-30 の 23:59:59.999 と 10-01 の 00:00 が分かれる。
        let body = assert_status(
            client.get(
                "/api/stats/brews?start=2026-09-30&end=2026-10-31&granularity=month&utc_offset_minutes=540",
            ),
            200,
        );
        let brews = assert_brews(&body);
        assert_eq!(brews.len(), 2, "{body}");
        assert_brew_period(&brews[0], "2026-09", 1, 7.5);
        assert_brew_period(&brews[1], "2026-10", 1, 8.0);

        // 開始日と終了日は端末のローカル時刻の日付として扱う (FR-18)。
        // 同じ日付でもオフセットが変われば区間が変わる。
        let body = assert_status(
            client.get("/api/stats/brews?start=2026-09-21&end=2026-09-21&granularity=day&utc_offset_minutes=-300"),
            200,
        );
        let brews = assert_brews(&body);
        assert_eq!(brews.len(), 1, "{body}");
        assert_brew_period(&brews[0], "2026-09-21", 2, 20.5);

        // UTC のオフセット 0 では、UTC の日付がそのまま区間になる。
        let body = assert_status(
            client.get(
                "/api/stats/brews?start=2026-09-20&end=2026-09-20&granularity=day&utc_offset_minutes=0",
            ),
            200,
        );
        let brews = assert_brews(&body);
        assert_eq!(brews.len(), 1, "{body}");
        assert_brew_period(&brews[0], "2026-09-20", 2, 12.0);
    }

    #[test]
    fn wrangler_stats_brews_ok_for_a_user_without_records() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());

        let body = assert_status(
            ApiClient::new(&base_url, Some(&data.empty_session))
                .get("/api/stats/brews?granularity=day&utc_offset_minutes=540"),
            200,
        );
        assert_eq!(body["brews"], json!([]), "{body}");
    }

    #[test]
    fn wrangler_stats_brews_rounds_the_dose_sum() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        // 0.1 + 0.2 は浮動小数点では 0.30000000000000004 になるため、合計は小数第 1 位に丸める。
        let body = assert_status(
            ApiClient::new(&base_url, Some(&data.rounding_session))
                .get("/api/stats/brews?granularity=day&utc_offset_minutes=0"),
            200,
        );
        let brews = body["brews"].as_array().expect("brews");
        assert_eq!(brews.len(), 1, "{body}");
        assert_eq!(brews[0]["period"], "2026-09-15", "{body}");
        assert_eq!(brews[0]["brew_count"], 2, "{body}");
        assert_eq!(brews[0]["dose_grams"], 0.3, "{body}");
    }

    #[test]
    fn wrangler_stats_accepts_an_encoded_plus_offset() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        // URL のクエリでは + は空白を表すため、符号付きのオフセットは %2B で送る。
        let body = assert_status(
            ApiClient::new(&base_url, Some(&data.rounding_session))
                .get("/api/stats/brews?granularity=day&utc_offset_minutes=%2B0"),
            200,
        );
        assert!(body["brews"].is_array(), "{body}");
    }

    #[test]
    fn wrangler_stats_brews_invalid_input_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.stats_session));

        for query in [
            // 粒度は必須で `day` と `month` だけを受け付ける (FR-18)。
            "utc_offset_minutes=540",
            "granularity=&utc_offset_minutes=540",
            "granularity=week&utc_offset_minutes=540",
            "granularity=DAY&utc_offset_minutes=540",
            "granularity=days&utc_offset_minutes=540",
            // オフセットは必須で -840 から 840 の整数だけを受け付ける (FR-18)。
            "granularity=day",
            "granularity=day&utc_offset_minutes=",
            "granularity=day&utc_offset_minutes=abc",
            "granularity=day&utc_offset_minutes=540.0",
            "granularity=day&utc_offset_minutes=841",
            "granularity=day&utc_offset_minutes=-841",
            "granularity=day&utc_offset_minutes=2147483648",
            // 期間は実在する `YYYY-MM-DD` だけを受け付け、開始日が終了日より後なら拒否する (FR-18)。
            "start=2026-02-30&granularity=day&utc_offset_minutes=540",
            "start=2026-9-21&granularity=day&utc_offset_minutes=540",
            "start=2026-09-21T00:00:00.000Z&granularity=day&utc_offset_minutes=540",
            "end=2026-13-01&granularity=day&utc_offset_minutes=540",
            "start=2026-09-22&end=2026-09-21&granularity=day&utc_offset_minutes=540",
            "start=&granularity=day&utc_offset_minutes=540",
        ] {
            assert_bad_request(client.get(&format!("/api/stats/brews?{query}")));
        }

        // 端 (840 と -840) と、同じ日の開始日と終了日は受け付ける。
        for query in [
            "granularity=day&utc_offset_minutes=840",
            "granularity=day&utc_offset_minutes=-840",
            "start=2026-09-21&end=2026-09-21&granularity=day&utc_offset_minutes=540",
            "granularity=month&utc_offset_minutes=0",
        ] {
            assert_status(client.get(&format!("/api/stats/brews?{query}")), 200);
        }
    }

    #[test]
    fn wrangler_stats_brews_unauthenticated_401() {
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(
            anonymous(&base_url).get("/api/stats/brews?granularity=day&utc_offset_minutes=540"),
        );
    }
}

mod stats_purchases {
    //! `GET /api/stats/purchases` のテスト (FR-18)。

    use super::*;

    #[test]
    fn wrangler_stats_purchases_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_session));

        // 通貨コードの昇順で並べ、null は先頭にする。価格と重量が NULL の購入は件数に数える (FR-18)。
        let body = assert_status(
            client.get("/api/stats/purchases?start=2026-09-21&end=2026-09-22&granularity=day"),
            200,
        );
        let purchases = assert_purchases(&body);
        assert_eq!(purchases.len(), 5, "{body}");
        assert_purchase_period(&purchases[0], "2026-09-21", None, 0, 0, 1);
        assert_purchase_period(&purchases[1], "2026-09-21", Some("JPY"), 1200, 200, 1);
        assert_purchase_period(&purchases[2], "2026-09-21", Some("USD"), 800, 150, 1);
        assert_purchase_period(&purchases[3], "2026-09-22", None, 0, 100, 1);
        assert_purchase_period(&purchases[4], "2026-09-22", Some("JPY"), 500, 0, 1);

        // 購入日はタイムゾーンを持たないため、オフセットを受け取らない (FR-18)。
        // 同じ期間を長い期間 (月別) で見ると、同じ区間にまとまる。
        let body = assert_status(client.get("/api/stats/purchases?granularity=month"), 200);
        let purchases = assert_purchases(&body);
        assert_eq!(purchases.len(), 4, "{body}");
        assert_purchase_period(&purchases[0], "2026-09", None, 0, 100, 2);
        assert_purchase_period(&purchases[1], "2026-09", Some("JPY"), 2700, 500, 3);
        assert_purchase_period(&purchases[2], "2026-09", Some("USD"), 800, 150, 1);
        assert_purchase_period(&purchases[3], "2026-10", Some("JPY"), 2000, 400, 1);

        // 開始日だけを指定する。記録の無い区間 (09-23 から 09-30) は返さない (FR-18)。
        let body = assert_status(
            client.get("/api/stats/purchases?start=2026-09-22&granularity=day"),
            200,
        );
        let purchases = assert_purchases(&body);
        assert_eq!(purchases.len(), 3, "{body}");
        assert_purchase_period(&purchases[0], "2026-09-22", None, 0, 100, 1);
        assert_purchase_period(&purchases[1], "2026-09-22", Some("JPY"), 500, 0, 1);
        assert_purchase_period(&purchases[2], "2026-10-01", Some("JPY"), 2000, 400, 1);

        // 終了日だけを指定する。
        let body = assert_status(
            client.get("/api/stats/purchases?end=2026-09-21&granularity=day"),
            200,
        );
        let purchases = assert_purchases(&body);
        assert_eq!(purchases.len(), 4, "{body}");
        assert_purchase_period(&purchases[0], "2026-09-20", Some("JPY"), 1000, 300, 1);
        assert_purchase_period(&purchases[1], "2026-09-21", None, 0, 0, 1);
        assert_purchase_period(&purchases[2], "2026-09-21", Some("JPY"), 1200, 200, 1);
        assert_purchase_period(&purchases[3], "2026-09-21", Some("USD"), 800, 150, 1);

        // 購入の API は utc_offset_minutes を受け取らないため、指定しても結果は変わらない。
        let with_offset = assert_status(
            client.get(
                "/api/stats/purchases?start=2026-09-21&end=2026-09-21&granularity=day&utc_offset_minutes=540",
            ),
            200,
        );
        let without_offset = assert_status(
            client.get("/api/stats/purchases?start=2026-09-21&end=2026-09-21&granularity=day"),
            200,
        );
        assert_eq!(with_offset, without_offset);
    }

    #[test]
    fn wrangler_stats_purchases_ok_for_a_user_without_records() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());

        let body = assert_status(
            ApiClient::new(&base_url, Some(&data.empty_session))
                .get("/api/stats/purchases?granularity=month"),
            200,
        );
        assert_eq!(body["purchases"], json!([]), "{body}");
    }

    #[test]
    fn wrangler_stats_purchases_invalid_input_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.purchase_session));

        for query in [
            "",
            "granularity=",
            "granularity=week",
            "granularity=month&start=2026-02-30",
            "granularity=month&end=2026-09-32",
            "granularity=month&start=2026-9-1",
            "granularity=month&start=2026-09-22&end=2026-09-21",
        ] {
            assert_bad_request(client.get(&format!("/api/stats/purchases?{query}")));
        }

        // オフセットは購入の API では必須でない (受け取らない)。
        assert_status(client.get("/api/stats/purchases?granularity=day"), 200);
    }

    #[test]
    fn wrangler_stats_purchases_unauthenticated_401() {
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).get("/api/stats/purchases?granularity=month"));
    }
}

mod stats_brew_ratings {
    //! `GET /api/stats/brew-ratings` のテスト (FR-18)。

    use super::*;

    #[test]
    fn wrangler_brew_ratings_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.stats_session));

        // 評価を持つ抽出だけを、抽出日時の昇順で返す。条件が NULL の項目は null で返す (FR-18)。
        let body = assert_status(
            client.get(
                "/api/stats/brew-ratings?start=2026-09-21&end=2026-09-21&utc_offset_minutes=540",
            ),
            200,
        );
        let ratings = body["brew_ratings"]
            .as_array()
            .unwrap_or_else(|| panic!("the response must have brew_ratings: {body}"));
        assert_eq!(ratings.len(), 2, "{body}");
        assert_eq!(ratings[0]["id"], data.brew_a.id, "{body}");
        assert_eq!(ratings[0]["dose_grams"], 12.0, "{body}");
        assert_eq!(ratings[0]["water_grams"], 200.0, "{body}");
        assert_eq!(ratings[0]["water_temp_c"], 92.5, "{body}");
        assert_eq!(ratings[0]["brew_time_seconds"], 150, "{body}");
        assert_eq!(ratings[0]["rating"], 5, "{body}");
        assert_eq!(ratings[1]["id"], data.brew_i.id, "{body}");
        assert_eq!(ratings[1]["dose_grams"], Value::Null, "{body}");
        assert_eq!(ratings[1]["water_grams"], 160.0, "{body}");
        assert_eq!(ratings[1]["rating"], 3, "{body}");
        // B (評価が NULL) と C (09-22 の区間) と G (アーカイブ済み) は含まれない。
        assert!(
            !ratings
                .iter()
                .any(|rating| rating["id"] == data.brew_b.id || rating["id"] == data.brew_c.id),
            "{body}"
        );

        // 全期間では、評価を持つ抽出を日時の昇順で全て返す。条件が NULL の項目は null になる。
        let body = assert_status(
            client.get("/api/stats/brew-ratings?utc_offset_minutes=540"),
            200,
        );
        let ratings = body["brew_ratings"]
            .as_array()
            .unwrap_or_else(|| panic!("the response must have brew_ratings: {body}"));
        let ids: Vec<&str> = ratings
            .iter()
            .filter_map(|rating| rating["id"].as_str())
            .collect();
        assert_eq!(
            ids,
            vec![
                data.brew_f.id.as_str(),
                data.brew_a.id.as_str(),
                data.brew_i.id.as_str(),
                data.brew_c.id.as_str(),
                data.brew_d.id.as_str(),
            ],
            "{body}"
        );
        // D (湯量と湯の温度と時間が NULL) は null のまま返す (FR-18)。
        assert_eq!(ratings[4]["dose_grams"], 7.5, "{body}");
        assert_eq!(ratings[4]["water_grams"], Value::Null, "{body}");
        assert_eq!(ratings[4]["water_temp_c"], Value::Null, "{body}");
        assert_eq!(ratings[4]["brew_time_seconds"], Value::Null, "{body}");
        assert_eq!(ratings[4]["rating"], 3, "{body}");

        // オフセット -300 では区間が変わり、C だけが返る。
        let body = assert_status(
            client.get(
                "/api/stats/brew-ratings?start=2026-09-21&end=2026-09-21&utc_offset_minutes=-300",
            ),
            200,
        );
        let ratings = body["brew_ratings"]
            .as_array()
            .unwrap_or_else(|| panic!("the response must have brew_ratings: {body}"));
        assert_eq!(ratings.len(), 1, "{body}");
        assert_eq!(ratings[0]["id"], data.brew_c.id, "{body}");
        assert_eq!(ratings[0]["rating"], 4, "{body}");
    }

    #[test]
    fn wrangler_brew_ratings_invalid_input_400() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.stats_session));

        for query in [
            "",
            "utc_offset_minutes=",
            "utc_offset_minutes=840.5",
            "utc_offset_minutes=1000",
            "utc_offset_minutes=-1000",
            "start=2026-02-30&utc_offset_minutes=540",
            "start=2026-09-22&end=2026-09-21&utc_offset_minutes=540",
        ] {
            assert_bad_request(client.get(&format!("/api/stats/brew-ratings?{query}")));
        }

        // 期間の端は省略できる (全期間)。
        assert_status(
            client.get("/api/stats/brew-ratings?utc_offset_minutes=540"),
            200,
        );
    }

    #[test]
    fn wrangler_brew_ratings_unauthenticated_401() {
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(
            anonymous(&base_url).get("/api/stats/brew-ratings?utc_offset_minutes=540"),
        );
    }
}

mod purchases_rating_history {
    //! `GET /api/purchases/<ID>/rating-history` のテスト (FR-18、FR-12、FR-5)。

    use super::*;

    #[test]
    fn wrangler_purchases_rating_history_ok() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.stats_session));

        // 評価を持つ抽出だけを、抽出日時の昇順で返す。期間で絞らない (FR-18)。
        let body = assert_status(
            client.get(&format!(
                "/api/purchases/{}/rating-history",
                data.stats_purchase_a.id
            )),
            200,
        );
        let ratings = body["ratings"]
            .as_array()
            .unwrap_or_else(|| panic!("the response must have ratings: {body}"));
        assert_eq!(ratings.len(), 2, "{body}");
        // 1 ミリ秒違いの 2 件も抽出日時の昇順で並ぶ。
        assert_eq!(ratings[0]["id"], data.brew_f.id, "{body}");
        assert_eq!(
            ratings[0]["brewed_at"], "2026-09-20T14:59:59.999Z",
            "{body}"
        );
        assert_eq!(ratings[0]["rating"], 2, "{body}");
        assert_eq!(ratings[1]["id"], data.brew_a.id, "{body}");
        assert_eq!(
            ratings[1]["brewed_at"], "2026-09-20T15:00:00.000Z",
            "{body}"
        );
        assert_eq!(ratings[1]["rating"], 5, "{body}");
        // 評価が NULL の B と、アーカイブ済みの G は含まれない。
        assert!(
            !ratings
                .iter()
                .any(|rating| rating["id"] == data.brew_b.id
                    || rating["id"] == data.brew_archived.id),
            "{body}"
        );

        // 期間で絞らないため、月をまたぐ 2 件も返る。
        let body = assert_status(
            client.get(&format!(
                "/api/purchases/{}/rating-history",
                data.stats_purchase_b.id
            )),
            200,
        );
        let ratings = body["ratings"]
            .as_array()
            .unwrap_or_else(|| panic!("the response must have ratings: {body}"));
        assert_eq!(ratings.len(), 2, "{body}");
        assert_eq!(ratings[0]["id"], data.brew_c.id, "{body}");
        assert_eq!(ratings[1]["id"], data.brew_d.id, "{body}");
        assert_eq!(
            ratings[1]["brewed_at"], "2026-09-30T14:59:59.999Z",
            "{body}"
        );

        // アーカイブ済みの購入も指定できる (単件取得と同じ扱い。FR-12)。
        let body = assert_status(
            client.get(&format!(
                "/api/purchases/{}/rating-history",
                data.stats_archived_purchase.id
            )),
            200,
        );
        let ratings = body["ratings"]
            .as_array()
            .unwrap_or_else(|| panic!("the response must have ratings: {body}"));
        assert_eq!(ratings.len(), 1, "{body}");
        assert_eq!(ratings[0]["id"], data.brew_i.id, "{body}");
        assert_eq!(ratings[0]["rating"], 3, "{body}");

        // 抽出が無い購入は空の並びになる (購入金額と重量の統計の利用者の購入を指定する)。
        let body = assert_status(
            ApiClient::new(&base_url, Some(&data.purchase_session)).get(&format!(
                "/api/purchases/{}/rating-history",
                data.purchase_jpy.id
            )),
            200,
        );
        assert_eq!(body["ratings"], json!([]), "{body}");
    }

    #[test]
    fn wrangler_purchases_rating_history_other_user_404() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        let client = ApiClient::new(&base_url, Some(&data.stats_session));

        // 他の利用者の購入の ID と存在しない ID は区別せず 404 を返す (FR-5、FR-18)。
        assert_not_found(client.get(&format!(
            "/api/purchases/{}/rating-history",
            data.other_purchase.id
        )));
        assert_not_found(client.get("/api/purchases/no-such-id/rating-history"));
        // 他の利用者から見ても、こちらの購入は 404 になる。
        assert_not_found(
            ApiClient::new(&base_url, Some(&data.other_session)).get(&format!(
                "/api/purchases/{}/rating-history",
                data.stats_purchase_a.id
            )),
        );
    }

    #[test]
    fn wrangler_purchases_rating_history_unauthenticated_401() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());
        assert_unauthorized(anonymous(&base_url).get(&format!(
            "/api/purchases/{}/rating-history",
            data.stats_purchase_a.id
        )));
    }
}

mod isolation {
    //! 他の利用者の記録とアーカイブ済みの記録の除外の検査 (FR-5、FR-12、FR-18)。

    use super::*;

    #[test]
    fn wrangler_stats_return_only_own_records_and_exclude_archived() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());

        // 他の利用者から見た統計には、こちらの記録が 1 件も入らない。
        // 他の利用者の記録は購入 1 件と抽出 1 件 (アーカイブされていない) だけである。
        let other = ApiClient::new(&base_url, Some(&data.other_session));
        let body = assert_status(
            other.get("/api/stats/brews?granularity=day&utc_offset_minutes=540"),
            200,
        );
        let brews = assert_brews(&body);
        assert_eq!(brews.len(), 1, "{body}");
        assert_brew_period(&brews[0], "2026-09-21", 1, 200.0);

        let body = assert_status(other.get("/api/stats/purchases?granularity=day"), 200);
        let purchases = assert_purchases(&body);
        assert_eq!(purchases.len(), 1, "{body}");
        assert_purchase_period(&purchases[0], "2026-09-21", Some("JPY"), 5000, 500, 1);

        let body = assert_status(
            other.get("/api/stats/brew-ratings?utc_offset_minutes=540"),
            200,
        );
        let ratings = body["brew_ratings"]
            .as_array()
            .unwrap_or_else(|| panic!("the response must have brew_ratings: {body}"));
        assert_eq!(ratings.len(), 1, "{body}");
        assert_eq!(ratings[0]["id"], data.other_brew.id, "{body}");

        // こちらの統計には、他の利用者の記録も、アーカイブ済みの記録も入らない。
        // 期待値は下ごしらえの記録だけから計算した値に一致する (アーカイブ済みの 9999 JPY と
        // 100.0 グラムの抽出、他の利用者の 5000 JPY と 200.0 グラムの抽出は入らない)。
        let stats = ApiClient::new(&base_url, Some(&data.stats_session));
        let body = assert_status(
            stats.get("/api/stats/brews?start=2026-09-01&end=2026-09-30&granularity=month&utc_offset_minutes=540"),
            200,
        );
        let brews = assert_brews(&body);
        assert_eq!(brews.len(), 1, "{body}");
        assert_brew_period(&brews[0], "2026-09", 6, 40.0);

        let purchases_client = ApiClient::new(&base_url, Some(&data.purchase_session));
        let body = assert_status(
            purchases_client.get("/api/stats/purchases?granularity=month"),
            200,
        );
        let purchases = assert_purchases(&body);
        assert_eq!(purchases.len(), 4, "{body}");
        // アーカイブ済みの 9999 JPY が入らないことを、合計の一致で確かめる。
        assert_purchase_period(&purchases[1], "2026-09", Some("JPY"), 2700, 500, 3);

        let body = assert_status(
            stats.get("/api/stats/brew-ratings?utc_offset_minutes=540"),
            200,
        );
        let ids: Vec<&str> = body["brew_ratings"]
            .as_array()
            .unwrap_or_else(|| panic!("the response must have brew_ratings: {body}"))
            .iter()
            .filter_map(|rating| rating["id"].as_str())
            .collect();
        // アーカイブ済みの G (rating 1) と他の利用者の抽出は入らない。
        assert_eq!(ids.len(), 5, "{body}");
        assert!(!ids.contains(&data.brew_archived.id.as_str()), "{body}");
        assert!(!ids.contains(&data.other_brew.id.as_str()), "{body}");
    }

    #[test]
    fn wrangler_stats_ignore_the_other_users_records_in_the_totals() {
        let data = data();
        let lease = server();
        let base_url = lease.use_server(|server| server.base_url());

        // 購入金額の統計は、同じ購入日の他の利用者の購入 (5000 JPY) を合計に入れない。
        let client = ApiClient::new(&base_url, Some(&data.purchase_session));
        let body = assert_status(
            client.get("/api/stats/purchases?start=2026-09-21&end=2026-09-21&granularity=day"),
            200,
        );
        let purchases = assert_purchases(&body);
        assert_eq!(purchases.len(), 3, "{body}");
        assert_purchase_period(&purchases[1], "2026-09-21", Some("JPY"), 1200, 200, 1);
        assert!(
            purchases
                .iter()
                .all(|purchase| purchase["price_amount"].as_i64() != Some(5000)),
            "the other user's purchase must not be summed: {body}"
        );
    }
}
