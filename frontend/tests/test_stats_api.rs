//! `records::stats` の単体テスト (0042)。
//!
//! 期間の切り替えが API に渡すパラメータの変換になること (完了条件 1) と、応答の型への
//! 変換を、偽の送信の実装で確かめる。

mod support;

use std::rc::Rc;

use brew_book_frontend::api::Method;
use brew_book_frontend::records::stats::{stats_path, BrewPeriod, BrewRating, RatingHistoryEntry};
use brew_book_frontend::records::stats_period::{
    stats_period_for, StatsGranularity, StatsPeriodPreset,
};
use brew_book_frontend::records::values::{LocalDate, LocalDateTime};
use brew_book_frontend::records::{RecordError, StatsApi};
use serde_json::{json, Value};

use support::{block_on, client, FakeTransport};

/// 統計のテストが使う端末 (2026-09-25、日本標準時の +540)。
fn test_now() -> LocalDateTime {
    LocalDateTime::new(2026, 9, 25, 12, 30)
}

/// 期間の切り替えから、抽出の API の経路を組み立てる。
fn brews_path(preset: StatsPeriodPreset, custom: Option<(LocalDate, LocalDate)>) -> String {
    let period = stats_period_for(preset, test_now(), custom).expect("the period must be built");
    stats_path(
        "/stats/brews",
        period.start.as_deref(),
        period.end.as_deref(),
        Some(period.granularity),
        Some(540),
    )
}

/// 応答を 1 つだけ返す統計の API クライアントを作る。
fn with_response(body: Value) -> (StatsApi, Rc<FakeTransport>) {
    let (client, transport) = client(vec![FakeTransport::response(200, &body.to_string())]);
    (StatsApi::new(client), transport)
}

#[test]
fn the_initial_period_is_the_current_month_daily() {
    // 初期状態は端末のタイムゾーンでの当月 1 日から当日までを日別で呼ぶ (FR-18)。
    assert_eq!(
        brews_path(StatsPeriodPreset::CurrentMonth, None),
        "/stats/brews?granularity=day&start=2026-09-01&end=2026-09-25&utc_offset_minutes=540"
    );
}

#[test]
fn the_period_switching_becomes_the_api_parameters() {
    // 3 か月、6 か月、12 か月は当月を含む直近の月数の月別にする (FR-18)。
    assert_eq!(
        brews_path(StatsPeriodPreset::ThreeMonths, None),
        "/stats/brews?granularity=month&start=2026-07-01&end=2026-09-25&utc_offset_minutes=540"
    );
    assert_eq!(
        brews_path(StatsPeriodPreset::SixMonths, None),
        "/stats/brews?granularity=month&start=2026-04-01&end=2026-09-25&utc_offset_minutes=540"
    );
    assert_eq!(
        brews_path(StatsPeriodPreset::TwelveMonths, None),
        "/stats/brews?granularity=month&start=2025-10-01&end=2026-09-25&utc_offset_minutes=540"
    );
    // 全期間は開始日と終了日の両方を省略する (FR-18)。
    assert_eq!(
        brews_path(StatsPeriodPreset::AllTime, None),
        "/stats/brews?granularity=month&utc_offset_minutes=540"
    );
}

#[test]
fn the_custom_period_switches_the_granularity_at_62_days() {
    // 2026-07-01 から 2026-08-31 までは 62 日なので日別。
    assert_eq!(
        brews_path(
            StatsPeriodPreset::Custom,
            Some((LocalDate::new(2026, 7, 1), LocalDate::new(2026, 8, 31)))
        ),
        "/stats/brews?granularity=day&start=2026-07-01&end=2026-08-31&utc_offset_minutes=540"
    );
    // 63 日以上は月別。
    assert_eq!(
        brews_path(
            StatsPeriodPreset::Custom,
            Some((LocalDate::new(2026, 7, 1), LocalDate::new(2026, 9, 1)))
        ),
        "/stats/brews?granularity=month&start=2026-07-01&end=2026-09-01&utc_offset_minutes=540"
    );
}

#[test]
fn the_purchases_path_has_no_utc_offset() {
    // 購入日はタイムゾーンを持たないため、オフセットを渡さない (FR-18)。
    assert_eq!(
        stats_path(
            "/stats/purchases",
            Some("2026-09-01"),
            Some("2026-09-25"),
            Some(StatsGranularity::Day),
            None
        ),
        "/stats/purchases?granularity=day&start=2026-09-01&end=2026-09-25"
    );
}

#[test]
fn the_brew_ratings_path_has_no_granularity() {
    // 抽出条件と評価の関係は粒度を持たない (FR-18)。
    assert_eq!(
        stats_path("/stats/brew-ratings", None, None, None, Some(540)),
        "/stats/brew-ratings?utc_offset_minutes=540"
    );
}

#[test]
fn the_brews_request_sends_the_query_and_reads_the_periods() {
    let (api, transport) = with_response(json!({
        "brews": [
            {"period": "2026-09-01", "brew_count": 3, "dose_grams": 45.0},
            {"period": "2026-09-02", "brew_count": 2, "dose_grams": 30.5},
        ],
    }));
    let periods = block_on(api.brews(
        Some("2026-09-01"),
        Some("2026-09-25"),
        StatsGranularity::Day,
        540,
    ))
    .expect("the periods must be read");
    assert_eq!(
        periods,
        vec![
            BrewPeriod {
                period: "2026-09-01".to_string(),
                brew_count: 3,
                dose_grams: 45.0,
            },
            BrewPeriod {
                period: "2026-09-02".to_string(),
                brew_count: 2,
                dose_grams: 30.5,
            },
        ]
    );
    let request = transport.last_request();
    assert_eq!(request.method, Method::Get);
    assert_eq!(
        request.path,
        "/api/stats/brews?granularity=day&start=2026-09-01&end=2026-09-25&utc_offset_minutes=540"
    );
}

#[test]
fn the_purchases_request_reads_the_currency_groups() {
    let (api, transport) = with_response(json!({
        "purchases": [
            {
                "period": "2026-09-01",
                "price_currency": null,
                "price_amount": 0,
                "weight_grams": 100,
                "purchase_count": 1,
            },
            {
                "period": "2026-09-01",
                "price_currency": "JPY",
                "price_amount": 1200,
                "weight_grams": 200,
                "purchase_count": 2,
            },
        ],
    }));
    let purchases = block_on(api.purchases(
        Some("2026-09-01"),
        Some("2026-09-25"),
        StatsGranularity::Day,
    ))
    .expect("the purchases must be read");
    assert_eq!(purchases.len(), 2);
    assert_eq!(purchases[0].price_currency, None);
    assert_eq!(purchases[1].price_currency.as_deref(), Some("JPY"));
    assert_eq!(purchases[1].price_amount, 1200);
    assert_eq!(purchases[1].weight_grams, 200);
    assert_eq!(purchases[1].purchase_count, 2);
    // オフセットは付けない (FR-18)。
    assert_eq!(
        transport.last_request().path,
        "/api/stats/purchases?granularity=day&start=2026-09-01&end=2026-09-25"
    );
}

#[test]
fn the_brew_ratings_request_reads_the_null_conditions() {
    let (api, transport) = with_response(json!({
        "brew_ratings": [
            {
                "id": "b1",
                "dose_grams": 12.0,
                "water_grams": 200.0,
                "water_temp_c": 92.5,
                "brew_time_seconds": 150,
                "rating": 5,
            },
            {
                "id": "b2",
                "dose_grams": null,
                "water_grams": 160.0,
                "water_temp_c": null,
                "brew_time_seconds": null,
                "rating": 3,
            },
        ],
    }));
    let ratings = block_on(api.brew_ratings(Some("2026-09-01"), Some("2026-09-25"), 540))
        .expect("the ratings must be read");
    assert_eq!(
        ratings,
        vec![
            BrewRating {
                id: "b1".to_string(),
                dose_grams: Some(12.0),
                water_grams: Some(200.0),
                water_temp_c: Some(92.5),
                brew_time_seconds: Some(150),
                rating: 5,
            },
            BrewRating {
                id: "b2".to_string(),
                dose_grams: None,
                water_grams: Some(160.0),
                water_temp_c: None,
                brew_time_seconds: None,
                rating: 3,
            },
        ]
    );
    assert_eq!(
        transport.last_request().path,
        "/api/stats/brew-ratings?start=2026-09-01&end=2026-09-25&utc_offset_minutes=540"
    );
}

#[test]
fn the_rating_history_request_reads_the_entries() {
    let (api, transport) = with_response(json!({
        "ratings": [
            {"id": "b1", "brewed_at": "2026-09-20T14:59:59.999Z", "rating": 2},
            {"id": "b2", "brewed_at": "2026-09-20T15:00:00.000Z", "rating": 4},
        ],
    }));
    let entries = block_on(api.rating_history("p1")).expect("the entries must be read");
    assert_eq!(
        entries,
        vec![
            RatingHistoryEntry {
                id: "b1".to_string(),
                brewed_at: "2026-09-20T14:59:59.999Z".to_string(),
                rating: 2,
            },
            RatingHistoryEntry {
                id: "b2".to_string(),
                brewed_at: "2026-09-20T15:00:00.000Z".to_string(),
                rating: 4,
            },
        ]
    );
    assert_eq!(
        transport.last_request().path,
        "/api/purchases/p1/rating-history"
    );
}

#[test]
fn a_malformed_response_is_a_format_error() {
    // 豆の量は必須 (応答の形式の違反で一覧を静かに打ち切らない)。
    let (api, _) = with_response(json!({
        "brews": [{"period": "2026-09-01", "brew_count": 3}],
    }));
    let error = block_on(api.brews(None, None, StatsGranularity::Month, 540))
        .expect_err("the response must be rejected");
    assert!(matches!(error, RecordError::Format(_)), "{error:?}");

    // 負の件数も拒否する (読み取り補助の共通の分岐。0042 のレビューの指摘)。
    let (api, _) = with_response(json!({
        "brews": [{"period": "2026-09-01", "brew_count": -1, "dose_grams": 12.0}],
    }));
    let error = block_on(api.brews(None, None, StatsGranularity::Month, 540))
        .expect_err("the response must be rejected");
    assert!(matches!(error, RecordError::Format(_)), "{error:?}");

    // 評価は 1 から 5 の外を拒否する。
    let (api, _) = with_response(json!({
        "brew_ratings": [{
            "id": "b1",
            "dose_grams": 12.0,
            "water_grams": null,
            "water_temp_c": null,
            "brew_time_seconds": null,
            "rating": 6,
        }],
    }));
    let error =
        block_on(api.brew_ratings(None, None, 540)).expect_err("the response must be rejected");
    assert!(matches!(error, RecordError::Format(_)), "{error:?}");
}
