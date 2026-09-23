//! 想定規模 (1 利用者あたり店 100 件、商品 1,000 件) のデータでの一覧と単件の取得の処理時間を測る。
//!
//! 応答時間の目標 (p95 200 ms) の合否は、リリース後に本番の Workers Logs の `duration_ms` の
//! 集計で確認する (PRD の成功指標)。このテストは処理時間を測って標準出力に出し、応答の
//! 正しさ (件数と並び順) だけを検査する。時間の上限は判定しない (機械の負荷で揺れるため)。
//!
//! テスト名の `wrangler_` は、`wrangler dev` を起動するテストを `backend:test` が名前で除外するための規約。

mod support;

use std::time::{Duration, Instant};

use coffee_log_core::datetime::format_epoch_millis;
use support::http::{read, ApiClient};
use support::seed::{user_id, Seed};

/// 店の件数 (想定規模)。
const SHOP_COUNT: usize = 100;
/// 商品の件数 (想定規模)。
const PRODUCT_COUNT: usize = 1_000;
/// 1 つの文に入れる行数。コマンドラインの長さを抑えるために分ける。
const ROWS_PER_STATEMENT: usize = 500;
/// 1 つの測定の繰り返し回数。
const MEASUREMENTS: usize = 10;
/// ページの上限 (PRD の性能)。
const MAX_PAGE_SIZE: usize = 200;

#[test]
fn wrangler_records_scale_of_the_list_and_the_single_fetch() {
    let scale = ScaleData::new();
    let lease = support::shared_server("scale", || {
        support::DevServer::start_with(|_| Vec::new(), &scale.seed_sql)
    })
    .expect("wrangler dev must start");
    let base_url = lease.use_server(|server| server.base_url());
    let client = ApiClient::new(&base_url, Some(&scale.session));
    lease.use_server(|server| {
        for statement in &scale.bulk_sql {
            server
                .execute_sql(statement)
                .expect("the bulk rows must be inserted");
        }
    });

    // 一覧 (既定の 50 件)、一覧 (上限の 200 件)、単件の取得を測る。
    let list_50 = measure(MEASUREMENTS, || {
        let body = get_json(&client, "/api/products");
        let products = body["products"].as_array().expect("products");
        assert_eq!(products.len(), 50, "the default page size is 50");
        // 作成日時の降順で返る。
        assert_eq!(
            products[0]["name"],
            format!("一覧の商品 {}", PRODUCT_COUNT - 1)
        );
        (body["next_cursor"].is_string(), 50)
    });
    let list_200 = measure(MEASUREMENTS, || {
        let body = get_json(&client, &format!("/api/products?limit={MAX_PAGE_SIZE}"));
        let products = body["products"].as_array().expect("products");
        assert_eq!(products.len(), MAX_PAGE_SIZE, "the page size must be 200");
        (true, MAX_PAGE_SIZE)
    });
    // 作成日時は添字とともに進むため、添字 0 が最も古く、最後の添字が最も新しい。
    let single_oldest = measure(MEASUREMENTS, || {
        let body = get_json(&client, &format!("/api/products/{}", row_id("product", 0)));
        assert_eq!(body["name"], "一覧の商品 0");
        (body["id"].is_string(), 1)
    });
    let single_newest = measure(MEASUREMENTS, || {
        let body = get_json(
            &client,
            &format!("/api/products/{}", row_id("product", PRODUCT_COUNT - 1)),
        );
        assert_eq!(body["name"], format!("一覧の商品 {}", PRODUCT_COUNT - 1));
        (body["id"].is_string(), 1)
    });
    let shops = measure(MEASUREMENTS, || {
        let body = get_json(&client, &format!("/api/shops?limit={MAX_PAGE_SIZE}"));
        let shops = body["shops"].as_array().expect("shops");
        assert_eq!(shops.len(), SHOP_COUNT, "the shops must be listed");
        assert_eq!(shops[0]["name"], format!("一覧の店 {}", SHOP_COUNT - 1));
        (true, SHOP_COUNT)
    });
    let single_shop = measure(MEASUREMENTS, || {
        let body = get_json(&client, &format!("/api/shops/{}", row_id("shop", 0)));
        assert_eq!(body["name"], "一覧の店 0");
        (body["id"].is_string(), 1)
    });

    println!(
        "the measurements of the assumed scale ({} shops, {} products):",
        SHOP_COUNT, PRODUCT_COUNT
    );
    report("products list (limit 50)", &list_50);
    report("products list (limit 200)", &list_200);
    report("products single (the oldest)", &single_oldest);
    report("products single (the newest)", &single_newest);
    report("shops list (100 shops)", &shops);
    report("shops single (1 shop)", &single_shop);
}

/// 想定規模の下ごしらえ。
struct ScaleData {
    seed_sql: String,
    bulk_sql: Vec<String>,
    session: String,
}

impl ScaleData {
    fn new() -> Self {
        let future = "2099-01-01T00:00:00.000Z";
        let created = "2026-09-01T00:00:00.000Z";
        let user = user_id(8);
        let mut seed = Seed::new();
        seed.user(&user, "scale user", created);
        let session = seed.session(&user, future, created);

        let mut bulk_sql = Vec::new();
        // 店 100 件。作成日時は 1 分ずつ進める。
        bulk_sql.extend(chunked_insert(
            "INSERT INTO shops (id, user_id, name, address, created_at, updated_at, archived_at)",
            (0..SHOP_COUNT)
                .map(|index| {
                    format!(
                        "({}, {}, {}, NULL, {}, {}, NULL)",
                        literal(&row_id("shop", index)),
                        literal(&user),
                        literal(&format!("一覧の店 {index}")),
                        literal(&timestamp(index)),
                        literal(&timestamp(index))
                    )
                })
                .collect(),
        ));
        // 商品 1,000 件。作成日時は 1 分ずつ進める。
        bulk_sql.extend(chunked_insert(
            "INSERT INTO products (id, user_id, name, producer, origin, region, process, variety, \
             created_at, updated_at, archived_at)",
            (0..PRODUCT_COUNT)
                .map(|index| {
                    format!(
                        "({}, {}, {}, NULL, NULL, NULL, NULL, NULL, {}, {}, NULL)",
                        literal(&row_id("product", index)),
                        literal(&user),
                        literal(&format!("一覧の商品 {index}")),
                        literal(&timestamp(index)),
                        literal(&timestamp(index))
                    )
                })
                .collect(),
        ));

        Self {
            seed_sql: seed.sql(),
            bulk_sql,
            session,
        }
    }
}

/// 測定の結果 (ミリ秒)。
struct Measurement {
    times: Vec<u128>,
    rows: usize,
}

/// 処理を `count` 回実行して、1 回あたりの処理時間を測る。正しさの検査は `action` が行う。
fn measure(count: usize, action: impl Fn() -> (bool, usize)) -> Measurement {
    // 温める (最初の 1 回はコネクションやキャッシュの影響を受ける)。
    let _ = action();
    let mut times = Vec::with_capacity(count);
    let mut rows = 0;
    for _ in 0..count {
        let started = Instant::now();
        let (ok, count) = action();
        assert!(ok, "the response must be correct");
        times.push(started.elapsed().as_millis());
        rows = count;
    }
    Measurement { times, rows }
}

/// 測定の結果を標準出力に出す。
fn report(label: &str, measurement: &Measurement) {
    let mut times = measurement.times.clone();
    times.sort_unstable();
    let min = times.first().copied().unwrap_or_default();
    let max = times.last().copied().unwrap_or_default();
    let median = times[times.len() / 2];
    println!(
        "{label}: {rows} rows, {count} runs, min {min} ms, median {median} ms, max {max} ms",
        rows = measurement.rows,
        count = times.len(),
    );
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

/// 行の ID。
fn row_id(kind: &str, index: usize) -> String {
    format!("scale-{kind}-{index:04}")
}

/// 想定規模の行の作成日時。基準の時刻から 1 分ずつ進める。
fn timestamp(index: usize) -> String {
    format_epoch_millis(1_800_000_000_000 + index as i64 * 60_000)
        .expect("the timestamp must format")
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
