//! 想定規模 (利用者 20 人、1 利用者あたり店 100 件、商品 1,000 件、購入 3,000 件、抽出 30,000 件) の
//! データでの統計の全期間の集計の処理時間を測る。
//!
//! 応答時間の目標 (統計の API の p95 は 500 ms) の合否は、リリース後に本番の Workers Logs の
//! `duration_ms` の集計で確認する (PRD の成功指標)。このテストは処理時間を測って標準出力に出し、
//! 応答の正しさ (区間の数と合計と並び順) だけを検査する。時間の上限は判定しない
//! (機械の負荷で揺れるため)。
//!
//! 統計の SQL は brews と purchases だけを読むが、データは想定規模の全体を投入する
//! (店と商品は購入の参照先として要る)。抽出日時は 1 時間ずつ、購入日は 1 日ずつ進める。
//! 区間の数が膨らみすぎないよう、購入日は 365 日の中で繰り返す。
//!
//! テスト名の `wrangler_` は、`wrangler dev` を起動するテストを `backend:test` が名前で除外するための規約。

mod support;

use std::time::{Duration, Instant};

use coffee_log_core::datetime::{format_epoch_millis, parse_epoch_millis};
use support::http::{read, ApiClient};
use support::seed::{user_id, Seed};

/// 利用者の数 (想定規模)。
const USER_COUNT: usize = 20;
/// 1 利用者あたりの店の件数 (想定規模)。
const SHOP_COUNT: usize = 100;
/// 1 利用者あたりの商品の件数 (想定規模)。
const PRODUCT_COUNT: usize = 1_000;
/// 1 利用者あたりの購入の件数 (想定規模)。
const PURCHASE_COUNT: usize = 3_000;
/// 1 利用者あたりの抽出の件数 (想定規模)。
const BREW_COUNT: usize = 30_000;
/// 1 つの購入に紐づける抽出の件数。
const BREWS_PER_PURCHASE: usize = BREW_COUNT / PURCHASE_COUNT;
/// 1 つの文に入れる行数。D1 の 1 文の長さの上限 (100 KB) を超えないように分ける。
const ROWS_PER_STATEMENT: usize = 400;
/// 1 つの測定の繰り返し回数。
const MEASUREMENTS: usize = 10;
/// 購入日を繰り返す日数 (1 年分)。
const PURCHASE_DAYS: usize = 365;
/// 1 時間のミリ秒。
const MILLIS_PER_HOUR: i64 = 3_600_000;
/// 1 日のミリ秒。
const MILLIS_PER_DAY: i64 = 86_400_000;
/// 日本標準時の UTC オフセット (分)。
const JST_OFFSET_MINUTES: i64 = 540;
/// 1 分のミリ秒。
const MILLIS_PER_MINUTE: i64 = 60_000;

#[test]
fn wrangler_stats_scale_of_the_full_period_aggregation() {
    let scale = ScaleData::new();
    let lease = support::shared_server("stats-scale", || {
        support::DevServer::start_with(|_| Vec::new(), &scale.seed_sql)
    })
    .expect("wrangler dev must start");
    let base_url = lease.use_server(|server| server.base_url());
    let client = ApiClient::new(&base_url, Some(&scale.session));
    lease.use_server(|server| {
        server
            .execute_sql_file(&scale.bulk_sql)
            .expect("the bulk rows must be inserted");
    });

    // 抽出回数と豆の消費量。全期間 (期間の端を省略する)。
    let brews_day = measure(MEASUREMENTS, || {
        let body = get_json(
            &client,
            "/api/stats/brews?granularity=day&utc_offset_minutes=540",
        );
        let brews = body["brews"].as_array().expect("brews");
        assert_eq!(
            brews.len(),
            BREW_COUNT / 24,
            "the local days of 30000 hours must be 1250"
        );
        assert_period_order(brews);
        let count = sum_integer(brews, "brew_count");
        assert_eq!(count, BREW_COUNT as i64, "every brew must be counted");
        let dose = sum_number(brews, "dose_grams");
        assert!(
            (dose - expected_dose_total()).abs() < 1.0,
            "the dose total must be about {}: {dose}",
            expected_dose_total()
        );
        brews.len()
    });
    let brews_month = measure(MEASUREMENTS, || {
        let body = get_json(
            &client,
            "/api/stats/brews?granularity=month&utc_offset_minutes=540",
        );
        let brews = body["brews"].as_array().expect("brews");
        assert_eq!(brews.len(), expected_brew_months());
        assert_period_order(brews);
        let count = sum_integer(brews, "brew_count");
        assert_eq!(count, BREW_COUNT as i64, "every brew must be counted");
        brews.len()
    });

    // 購入金額と重量。全期間 (期間の端を省略する)。
    let purchases_day = measure(MEASUREMENTS, || {
        let body = get_json(&client, "/api/stats/purchases?granularity=day");
        let purchases = body["purchases"].as_array().expect("purchases");
        assert_eq!(purchases.len(), PURCHASE_DAYS * 2);
        assert_currency_order(purchases);
        let count = sum_integer(purchases, "purchase_count");
        assert_eq!(
            count, PURCHASE_COUNT as i64,
            "every purchase must be counted"
        );
        assert_purchase_totals(purchases);
        purchases.len()
    });
    let purchases_month = measure(MEASUREMENTS, || {
        let body = get_json(&client, "/api/stats/purchases?granularity=month");
        let purchases = body["purchases"].as_array().expect("purchases");
        assert_eq!(
            purchases.len(),
            12 * 2,
            "one year of months and 2 currencies"
        );
        assert_currency_order(purchases);
        let count = sum_integer(purchases, "purchase_count");
        assert_eq!(
            count, PURCHASE_COUNT as i64,
            "every purchase must be counted"
        );
        assert_purchase_totals(purchases);
        purchases.len()
    });

    // 抽出条件と評価の関係 (散布図)。全期間では評価を持つ抽出を全て返す。
    let brew_ratings = measure(MEASUREMENTS, || {
        let body = get_json(&client, "/api/stats/brew-ratings?utc_offset_minutes=540");
        let ratings = body["brew_ratings"].as_array().expect("brew_ratings");
        let expected = BREW_COUNT - BREW_COUNT / 3;
        assert_eq!(ratings.len(), expected, "the rated brews must be listed");
        assert!(
            ratings
                .iter()
                .all(|rating| rating["rating"].as_i64().is_some()),
            "every row must carry a rating"
        );
        ratings.len()
    });

    // 購入ごとの評価の推移。1 つの購入に紐づく 10 件の抽出を返す。
    let rating_history = measure(MEASUREMENTS, || {
        let purchase_id = row_id("purchase", 0, USER_COUNT);
        let body = get_json(
            &client,
            &format!("/api/purchases/{purchase_id}/rating-history"),
        );
        let ratings = body["ratings"].as_array().expect("ratings");
        // 評価を持つのは、3 で割り切れない添字の抽出だけである。
        let expected = (0..BREWS_PER_PURCHASE).filter(|brew| brew % 3 != 0).count();
        assert_eq!(ratings.len(), expected);
        assert!(
            ratings
                .windows(2)
                .all(|pair| { pair[0]["brewed_at"].as_str() < pair[1]["brewed_at"].as_str() }),
            "the ratings must be ordered by the brewed timestamp"
        );
        ratings.len()
    });

    println!(
        "the measurements of the assumed scale ({} users, {} shops, {} products, {} purchases, \
         {} brews per user):",
        USER_COUNT, SHOP_COUNT, PRODUCT_COUNT, PURCHASE_COUNT, BREW_COUNT
    );
    report("brews stats (day, all)", &brews_day);
    report("brews stats (month, all)", &brews_month);
    report("purchases stats (day, all)", &purchases_day);
    report("purchases stats (month, all)", &purchases_month);
    report("brew ratings (all)", &brew_ratings);
    report("rating history (10 brews)", &rating_history);
}

/// 想定規模の下ごしらえ。
struct ScaleData {
    seed_sql: String,
    bulk_sql: Vec<String>,
    session: String,
}

impl ScaleData {
    fn new() -> Self {
        let created = "2026-09-01T00:00:00.000Z";
        let future = "2099-01-01T00:00:00.000Z";
        let user = user_id(USER_COUNT as u32);
        let mut seed = Seed::new();
        seed.user(&user, "stats scale user", created);
        let session = seed.session(&user, future, created);

        let mut bulk_sql = Vec::new();
        // 抽出日時は日本標準時の端末の 00:00 から始まる基準、購入日は日付だけの基準を使う。
        let base = base_millis();
        let purchases_base = purchases_base_millis();
        // 店と商品と購入と抽出を利用者ごとに投入する。測定の対象は最後の利用者だが、
        // 想定規模の全体 (利用者 20 人) を入れて、利用者 ID の条件が効くことを確かめる。
        for index in 1..=USER_COUNT {
            let user_id = user_id(index as u32);
            // 測定の対象の利用者は Seed が先に入れる (セッションの外部キーのため)。
            if index != USER_COUNT {
                bulk_sql.push(format!(
                    "INSERT INTO users (id, display_name, created_at) VALUES ({}, {}, {})",
                    literal(&user_id),
                    literal(&format!("scale user {index}")),
                    literal(created)
                ));
            }
            bulk_sql.extend(chunked_insert(
                "INSERT INTO shops (id, user_id, name, address, created_at, updated_at, archived_at)",
                (0..SHOP_COUNT)
                    .map(|shop| {
                        format!(
                            "({}, {}, {}, NULL, {}, {}, NULL)",
                            literal(&row_id("shop", shop, index)),
                            literal(&user_id),
                            literal(&format!("一覧の店 {shop}")),
                            literal(&timestamp(base, shop as i64 * MILLIS_PER_MINUTE)),
                            literal(&timestamp(base, shop as i64 * MILLIS_PER_MINUTE))
                        )
                    })
                    .collect(),
            ));
            bulk_sql.extend(chunked_insert(
                "INSERT INTO products (id, user_id, name, producer, origin, region, process, \
                 variety, created_at, updated_at, archived_at)",
                (0..PRODUCT_COUNT)
                    .map(|product| {
                        format!(
                            "({}, {}, {}, NULL, NULL, NULL, NULL, NULL, {}, {}, NULL)",
                            literal(&row_id("product", product, index)),
                            literal(&user_id),
                            literal(&format!("一覧の商品 {product}")),
                            literal(&timestamp(base, product as i64 * MILLIS_PER_MINUTE)),
                            literal(&timestamp(base, product as i64 * MILLIS_PER_MINUTE))
                        )
                    })
                    .collect(),
            ));
            bulk_sql.extend(chunked_insert(
                "INSERT INTO purchases (id, user_id, product_id, shop_id, purchased_on, roast, \
                 roast_date, price_amount, price_currency, weight_grams, photo_key, created_at, \
                 updated_at, archived_at)",
                (0..PURCHASE_COUNT)
                    .map(|purchase| {
                        format!(
                            "({}, {}, {}, NULL, {}, NULL, NULL, {}, {}, {}, NULL, {}, {}, NULL)",
                            literal(&row_id("purchase", purchase, index)),
                            literal(&user_id),
                            literal(&row_id("product", purchase % PRODUCT_COUNT, index)),
                            literal(&purchase_day(purchases_base, purchase % PURCHASE_DAYS)),
                            literal(&(100 + (purchase % 900)).to_string()),
                            literal(if purchase % 2 == 0 { "JPY" } else { "USD" }),
                            literal(&weight_grams()),
                            literal(&timestamp(
                                purchases_base,
                                purchase as i64 * MILLIS_PER_MINUTE
                            )),
                            literal(&timestamp(
                                purchases_base,
                                purchase as i64 * MILLIS_PER_MINUTE
                            ))
                        )
                    })
                    .collect(),
            ));
            bulk_sql.extend(chunked_insert(
                "INSERT INTO brews (id, user_id, purchase_id, brewed_at, dose_grams, water_grams, \
                 water_temp_c, brew_time_seconds, method, grind_setting, rating, notes, created_at, \
                 updated_at, archived_at)",
                (0..BREW_COUNT)
                    .map(|brew| {
                        format!(
                            "({}, {}, {}, {}, {}, NULL, NULL, NULL, NULL, NULL, {}, NULL, {}, {}, NULL)",
                            literal(&row_id("brew", brew, index)),
                            literal(&user_id),
                            literal(&row_id(
                                "purchase",
                                (brew / BREWS_PER_PURCHASE) % PURCHASE_COUNT,
                                index
                            )),
                            literal(&timestamp(base, brew as i64 * MILLIS_PER_HOUR)),
                            literal(&format!("{:.1}", (brew % 250) as f64 / 10.0)),
                            rating(brew),
                            literal(&timestamp(base, brew as i64 * MILLIS_PER_HOUR)),
                            literal(&timestamp(base, brew as i64 * MILLIS_PER_HOUR))
                        )
                    })
                    .collect(),
            ));
        }

        Self {
            seed_sql: seed.sql(),
            bulk_sql,
            session,
        }
    }
}

/// 想定規模の抽出日時の基準の時刻。`2027-01-01T00:00:00.000Z` の 9 時間前とし、
/// 日本標準時 (+540) の端末では `2027-01-01` の 00:00 になるようにする。
fn base_millis() -> i64 {
    parse_epoch_millis("2027-01-01T00:00:00.000Z").expect("the base must parse")
        - JST_OFFSET_MINUTES * MILLIS_PER_MINUTE
}

/// 想定規模の購入日の基準の日付 (`2027-01-01`)。購入日はタイムゾーンを持たない。
fn purchases_base_millis() -> i64 {
    parse_epoch_millis("2027-01-01T00:00:00.000Z").expect("the base must parse")
}

/// 想定規模の行の抽出日時。基準の時刻から 1 時間ずつ進める。
fn timestamp(base: i64, offset_millis: i64) -> String {
    format_epoch_millis(base + offset_millis).expect("the timestamp must format")
}

/// 想定規模の行の購入日 (基準の日から `days` 日後)。
fn purchase_day(base: i64, days: usize) -> String {
    timestamp(base, days as i64 * MILLIS_PER_DAY)[..10].to_owned()
}

/// 想定規模の行の重量 (グラム)。
fn weight_grams() -> String {
    200.to_string()
}

/// 抽出の評価。3 回に 1 回は評価を持たない。
fn rating(brew: usize) -> String {
    if brew.is_multiple_of(3) {
        "NULL".to_owned()
    } else {
        (1 + brew % 5).to_string()
    }
}

/// 評価を持つ抽出の豆の量の合計の期待値。
fn expected_dose_total() -> f64 {
    // 豆の量は 0.0 から 24.9 まで (小数第 1 位) の 250 周期の繰り返しになる。
    let cycle: f64 = (0..250).map(|value| value as f64 / 10.0).sum();
    cycle * (BREW_COUNT as f64 / 250.0)
}

/// 全期間の抽出のローカル月数 (+540) の期待値。
fn expected_brew_months() -> usize {
    let base = base_millis();
    let first = timestamp(base, JST_OFFSET_MINUTES * MILLIS_PER_MINUTE);
    let last = timestamp(
        base,
        (BREW_COUNT as i64 - 1) * MILLIS_PER_HOUR + JST_OFFSET_MINUTES * MILLIS_PER_MINUTE,
    );
    let month = |text: &str| {
        text[..4].parse::<i64>().expect("the year must parse") * 12
            + text[5..7].parse::<i64>().expect("the month must parse")
    };
    (month(&last) - month(&first) + 1) as usize
}

/// 行の ID。利用者ごとに一意にする。
fn row_id(kind: &str, index: usize, user: usize) -> String {
    format!("stats-{kind}-{user:02}-{index:04}")
}

/// 行を `ROWS_PER_STATEMENT` 件ずつの INSERT 文にする。
fn chunked_insert(prefix: &str, rows: Vec<String>) -> Vec<String> {
    rows.chunks(ROWS_PER_STATEMENT)
        .map(|chunk| format!("{prefix} VALUES {}", chunk.join(", ")))
        .collect()
}

/// 値を SQL のリテラルにする。テスト専用の値だけを渡す。
fn literal(value: &str) -> String {
    assert!(
        !value.contains('\''),
        "the test value must not contain a single quote: {value}"
    );
    format!("'{value}'")
}

/// 測定の結果 (ミリ秒)。
struct Measurement {
    times: Vec<u128>,
    rows: usize,
}

/// 処理を `count` 回実行して、1 回あたりの処理時間を測る。正しさの検査は `action` が行う。
fn measure(count: usize, action: impl Fn() -> usize) -> Measurement {
    // 温める (最初の 1 回はコネクションやキャッシュの影響を受ける)。
    let _ = action();
    let mut times = Vec::with_capacity(count);
    let mut rows = 0;
    for _ in 0..count {
        let started = Instant::now();
        let row_count = action();
        times.push(started.elapsed().as_millis());
        rows = row_count;
    }
    Measurement { times, rows }
}

/// 測定の結果を標準出力に出す。
fn report(label: &str, measurement: &Measurement) {
    let mut times = measurement.times.clone();
    times.sort_unstable();
    let min = times.first().copied().unwrap_or_default();
    let max = times.last().copied().unwrap_or_default();
    // 回数が偶数のときは中央の 2 つの平均を取る。
    let median = if times.is_empty() {
        0.0
    } else if times.len().is_multiple_of(2) {
        (times[times.len() / 2 - 1] + times[times.len() / 2]) as f64 / 2.0
    } else {
        times[times.len() / 2] as f64
    };
    println!(
        "{label}: {rows} rows, {count} runs, min {min} ms, median {median} ms, max {max} ms",
        rows = measurement.rows,
        count = times.len(),
    );
}

/// 区間のキーが昇順で重複しないことを確かめる。
fn assert_period_order(rows: &[serde_json::Value]) {
    let periods: Vec<&str> = rows
        .iter()
        .filter_map(|row| row["period"].as_str())
        .collect();
    assert_eq!(periods.len(), rows.len(), "every row must have a period");
    assert!(
        periods.windows(2).all(|pair| pair[0] < pair[1]),
        "the periods must be ascending and unique"
    );
}

/// 区間と通貨コードの組が昇順 (null は先頭) で重複しないことを確かめる。
fn assert_currency_order(rows: &[serde_json::Value]) {
    let keys: Vec<(String, Option<&str>)> = rows
        .iter()
        .map(|row| {
            (
                row["period"].as_str().expect("the period").to_owned(),
                row["price_currency"].as_str(),
            )
        })
        .collect();
    assert!(
        keys.windows(2).all(|pair| {
            pair[0].0 < pair[1].0 || (pair[0].0 == pair[1].0 && pair[0].1 < pair[1].1)
        }),
        "the periods and the currencies must be ascending and unique"
    );
}

/// 購入の合計が、下ごしらえの値と一致することを確かめる。
fn assert_purchase_totals(rows: &[serde_json::Value]) {
    let price = sum_integer(rows, "price_amount");
    let weight = sum_integer(rows, "weight_grams");
    assert_eq!(price, expected_price_total(), "the price total must match");
    assert_eq!(
        weight,
        PURCHASE_COUNT as i64 * 200,
        "the weight total must match"
    );
}

/// 価格の合計の期待値 (`100 + j % 900` の和)。
fn expected_price_total() -> i64 {
    (0..PURCHASE_COUNT)
        .map(|purchase| 100 + (purchase % 900) as i64)
        .sum()
}

/// 応答の整数の項目の合計。
fn sum_integer(rows: &[serde_json::Value], name: &str) -> i64 {
    rows.iter()
        .map(|row| {
            row[name]
                .as_i64()
                .unwrap_or_else(|| panic!("{name} must be an integer: {row}"))
        })
        .sum()
}

/// 応答の小数の項目の合計。
fn sum_number(rows: &[serde_json::Value], name: &str) -> f64 {
    rows.iter()
        .map(|row| {
            row[name]
                .as_f64()
                .unwrap_or_else(|| panic!("{name} must be a number: {row}"))
        })
        .sum()
}

/// API を GET して JSON を返す。
fn get_json(client: &ApiClient, path: &str) -> serde_json::Value {
    let mut last = (0_u16, serde_json::Value::Null);
    for _ in 0..3 {
        let (status, body) = read(client.get(path));
        if status == 200 {
            return body;
        }
        last = (status, body);
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("the request to {path} must succeed but was {last:?}");
}
