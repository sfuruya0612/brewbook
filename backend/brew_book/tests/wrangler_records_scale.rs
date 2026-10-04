//! 想定規模 (1 利用者あたり店 100 件、商品 1,000 件、購入 3,000 件、抽出 30,000 件) のデータでの
//! 一覧と単件の取得と、全記録のエクスポートの処理時間を測る。
//!
//! 応答時間の目標 (p95 200 ms) の合否は、リリース後に本番の Workers Logs の `duration_ms` の
//! 集計で確認する (PRD の成功指標)。このテストは処理時間を測って標準出力に出し、応答の
//! 正しさ (件数と並び順) だけを検査する。時間の上限は判定しない (機械の負荷で揺れるため)。
//! エクスポートは全行を Worker のメモリに載せるため、想定規模でも成功すること (FR-14) と、
//! 応答の JSON の大きさを併せて確認する。
//!
//! テスト名の `wrangler_` は、`wrangler dev` を起動するテストを `backend:test` が名前で除外するための規約。

mod support;

use std::time::{Duration, Instant};

use brew_book_core::datetime::format_epoch_millis;
use support::http::{read, ApiClient};
use support::seed::{user_id, Seed};

/// 店の件数 (想定規模)。
const SHOP_COUNT: usize = 100;
/// 商品の件数 (想定規模)。
const PRODUCT_COUNT: usize = 1_000;
/// 購入の件数 (想定規模)。
const PURCHASE_COUNT: usize = 3_000;
/// 抽出の件数 (想定規模)。
const BREW_COUNT: usize = 30_000;
/// 1 つの文に入れる行数。D1 の 1 文の長さの上限 (100 KB) を超えないように分ける。
const ROWS_PER_STATEMENT: usize = 400;
/// 1 つの測定の繰り返し回数。
const MEASUREMENTS: usize = 10;
/// エクスポートの測定の繰り返し回数。応答が大きいため回数を抑える。
const EXPORT_MEASUREMENTS: usize = 3;
/// ページの上限 (PRD の性能)。
const MAX_PAGE_SIZE: usize = 200;
/// 想定規模の計測の基準の時刻 (2027-01-15T08:00:00.000Z 相当)。
const BASE_MILLIS: i64 = 1_800_000_000_000;
/// 1 日のミリ秒。
const MILLIS_PER_DAY: i64 = 86_400_000;

#[test]
fn wrangler_records_scale_of_the_list_the_single_fetch_and_the_export() {
    let scale = ScaleData::new();
    let lease = support::shared_server("scale", || {
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
    // 購入日は添字とともに進むため、購入日の降順では最後の添字が最も新しく、添字 0 が最も古い。
    let purchases_list = measure(MEASUREMENTS, || {
        let body = get_json(&client, "/api/purchases");
        let purchases = body["purchases"].as_array().expect("purchases");
        assert_eq!(purchases.len(), 50, "the default page size is 50");
        assert_eq!(purchases[0]["id"], row_id("purchase", PURCHASE_COUNT - 1));
        // 応答には商品と店がネストする (結合の結果)。
        assert!(purchases[0]["product"]["id"].is_string(), "{body}");
        (body["next_cursor"].is_string(), 50)
    });
    let purchases_list_200 = measure(MEASUREMENTS, || {
        let body = get_json(&client, &format!("/api/purchases?limit={MAX_PAGE_SIZE}"));
        let purchases = body["purchases"].as_array().expect("purchases");
        assert_eq!(purchases.len(), MAX_PAGE_SIZE, "the page size must be 200");
        (true, MAX_PAGE_SIZE)
    });
    let purchase_oldest = measure(MEASUREMENTS, || {
        let body = get_json(
            &client,
            &format!("/api/purchases/{}", row_id("purchase", 0)),
        );
        assert_eq!(body["purchased_on"], day(0));
        (body["id"].is_string(), 1)
    });
    let purchase_newest = measure(MEASUREMENTS, || {
        let body = get_json(
            &client,
            &format!("/api/purchases/{}", row_id("purchase", PURCHASE_COUNT - 1)),
        );
        assert_eq!(body["purchased_on"], day(PURCHASE_COUNT - 1));
        (body["id"].is_string(), 1)
    });
    // 抽出は抽出日時の降順で返るため、添字 0 が最も新しい。
    let brews_list = measure(MEASUREMENTS, || {
        let body = get_json(&client, "/api/brews");
        let brews = body["brews"].as_array().expect("brews");
        assert_eq!(brews.len(), 50, "the default page size is 50");
        assert_eq!(brews[0]["id"], row_id("brew", BREW_COUNT - 1));
        // 応答には購入と、その中の商品と店がネストする (結合の結果)。
        assert!(brews[0]["purchase"]["product"]["id"].is_string(), "{body}");
        (body["next_cursor"].is_string(), 50)
    });
    let brews_list_200 = measure(MEASUREMENTS, || {
        let body = get_json(&client, &format!("/api/brews?limit={MAX_PAGE_SIZE}"));
        let brews = body["brews"].as_array().expect("brews");
        assert_eq!(brews.len(), MAX_PAGE_SIZE, "the page size must be 200");
        (true, MAX_PAGE_SIZE)
    });
    let brew_oldest = measure(MEASUREMENTS, || {
        let body = get_json(&client, &format!("/api/brews/{}", row_id("brew", 0)));
        assert_eq!(body["brewed_at"], timestamp(0));
        (body["id"].is_string(), 1)
    });
    let brew_newest = measure(MEASUREMENTS, || {
        let body = get_json(
            &client,
            &format!("/api/brews/{}", row_id("brew", BREW_COUNT - 1)),
        );
        assert_eq!(body["brewed_at"], timestamp(BREW_COUNT - 1));
        (body["id"].is_string(), 1)
    });

    // 全記録のエクスポート (FR-14)。想定規模の全行を 1 つの JSON に組み立てられること
    // (Worker のメモリに収まること) を確認する。
    let export = measure(EXPORT_MEASUREMENTS, || {
        let body = get_json(&client, "/api/export");
        assert_eq!(
            body["shops"].as_array().map(Vec::len),
            Some(SHOP_COUNT),
            "the export must carry every shop"
        );
        assert_eq!(
            body["products"].as_array().map(Vec::len),
            Some(PRODUCT_COUNT),
            "the export must carry every product"
        );
        assert_eq!(
            body["purchases"].as_array().map(Vec::len),
            Some(PURCHASE_COUNT),
            "the export must carry every purchase"
        );
        assert_eq!(
            body["brews"].as_array().map(Vec::len),
            Some(BREW_COUNT),
            "the export must carry every brew"
        );
        (
            true,
            SHOP_COUNT + PRODUCT_COUNT + PURCHASE_COUNT + BREW_COUNT,
        )
    });
    // 応答の JSON の大きさ。Worker が載せるメモリの見積もりの根拠にする。
    let export_bytes = serde_json::to_string(&get_json(&client, "/api/export"))
        .expect("the export must serialize")
        .len();

    println!(
        "the measurements of the assumed scale ({} shops, {} products, {} purchases, {} brews):",
        SHOP_COUNT, PRODUCT_COUNT, PURCHASE_COUNT, BREW_COUNT
    );
    report("products list (limit 50)", &list_50);
    report("products list (limit 200)", &list_200);
    report("products single (the oldest)", &single_oldest);
    report("products single (the newest)", &single_newest);
    report("shops list (100 shops)", &shops);
    report("shops single (1 shop)", &single_shop);
    report("purchases list (limit 50)", &purchases_list);
    report("purchases list (limit 200)", &purchases_list_200);
    report("purchases single (the oldest)", &purchase_oldest);
    report("purchases single (the newest)", &purchase_newest);
    report("brews list (limit 50)", &brews_list);
    report("brews list (limit 200)", &brews_list_200);
    report("brews single (the oldest)", &brew_oldest);
    report("brews single (the newest)", &brew_newest);
    report("export (all records)", &export);
    println!(
        "the export of the assumed scale is {} bytes of JSON ({} rows)",
        export_bytes, export.rows
    );
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
            "INSERT INTO shops (id, user_id, name, address, created_at, updated_at)",
            (0..SHOP_COUNT)
                .map(|index| {
                    format!(
                        "({}, {}, {}, NULL, {}, {})",
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
             created_at, updated_at)",
            (0..PRODUCT_COUNT)
                .map(|index| {
                    format!(
                        "({}, {}, {}, NULL, NULL, NULL, NULL, NULL, {}, {})",
                        literal(&row_id("product", index)),
                        literal(&user),
                        literal(&format!("一覧の商品 {index}")),
                        literal(&timestamp(index)),
                        literal(&timestamp(index))
                    )
                })
                .collect(),
        ));
        // 購入 3,000 件。購入日は基準の日から 1 日ずつ進める。
        bulk_sql.extend(chunked_insert(
            "INSERT INTO purchases (id, user_id, product_id, shop_id, purchased_on, roast, \
             roast_date, price_amount, price_currency, weight_grams, photo_key, created_at, \
             updated_at)",
            (0..PURCHASE_COUNT)
                .map(|index| {
                    format!(
                        "({}, {}, {}, NULL, {}, NULL, NULL, NULL, NULL, NULL, NULL, {}, {})",
                        literal(&row_id("purchase", index)),
                        literal(&user),
                        literal(&row_id("product", index % PRODUCT_COUNT)),
                        literal(&day(index)),
                        literal(&timestamp(index)),
                        literal(&timestamp(index))
                    )
                })
                .collect(),
        ));
        // 抽出 30,000 件。抽出日時は 1 分ずつ進める。
        bulk_sql.extend(chunked_insert(
            "INSERT INTO brews (id, user_id, purchase_id, brewed_at, dose_grams, water_grams, \
             water_temp_c, brew_time_seconds, method, grind_setting, rating, notes, created_at, \
             updated_at)",
            (0..BREW_COUNT)
                .map(|index| {
                    format!(
                        "({}, {}, {}, {}, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, {}, {})",
                        literal(&row_id("brew", index)),
                        literal(&user),
                        literal(&row_id("purchase", index % PURCHASE_COUNT)),
                        literal(&timestamp(index)),
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
    format_epoch_millis(BASE_MILLIS + index as i64 * 60_000).expect("the timestamp must format")
}

/// 想定規模の行の購入日 (YYYY-MM-DD)。基準の日から 1 日ずつ進める。
fn day(index: usize) -> String {
    let timestamp = format_epoch_millis(BASE_MILLIS + index as i64 * MILLIS_PER_DAY)
        .expect("the day must format");
    timestamp[..10].to_owned()
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
