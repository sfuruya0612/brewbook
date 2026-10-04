//! 統計のグラフのブラウザテスト (0042)。
//!
//! グラフを実際に描き、SVG の要素と計算済みスタイルを検査する。色は `chart-count` と
//! `chart-grams` の 2 つだけであること (docs/design/components/Charts) を原本の値と比べる。
//! 画面はルーターの文脈を要求するため、テスト用の経路と `MemoryHistory` で統計の画面を描き、
//! 初期状態の読み込みと当月の棒グラフも確かめる。期間の切り替えのパラメータの変換は native の
//! テスト (`test_stats_api.rs`) が担う。

#![cfg(target_arch = "wasm32")]

mod support;

use std::cell::RefCell;
use std::rc::Rc;

use brew_book_frontend::api::ApiClient;
use brew_book_frontend::auth::{AuthServices, SessionStatus};
use brew_book_frontend::i18n::{set_language, text, Key, Language};
use brew_book_frontend::records::stats::{
    BrewPeriod, BrewRating, PurchasePeriod, RatingHistoryEntry,
};
use brew_book_frontend::records::values::LocalDateTime;
use brew_book_frontend::records::{PhotoUploader, RecordServices, RecordsApi};
use brew_book_frontend::screens::stats::{
    BrewRatingScatterCharts, BrewStatsCharts, PurchaseStatsCharts, RatingHistoryChart,
};
use brew_book_frontend::screens::StatsScreen;
use dioxus::history::{History, MemoryHistory};
use dioxus::prelude::*;
use dioxus_router::components::HistoryProvider;
use dioxus_router::{Routable, Router};
use serde_json::json;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

use support::web::{computed, count, install_styles, mount, select, set_theme, tick};
use support::{
    FakeClock, FakeImageConverter, FakePasskeyClient, FakePhotoPicker, FakeTransport,
    FakeUploadTransport,
};

wasm_bindgen_test_configure!(run_in_browser);

/// Paper の chart-count (--roast #4a2f1c)。
const CHART_COUNT: &str = "rgb(74, 47, 28)";
/// Paper の chart-grams (--crema #b8742a)。
const CHART_GRAMS: &str = "rgb(184, 116, 42)";
/// Paper の paper-raised (#fcf8f1)。折れ線の点の塗り。
const PAPER_RAISED: &str = "rgb(252, 248, 241)";

thread_local! {
    /// 統計の画面のテストが使う偽の送信の実装 (要求を確かめる)。
    static STATS_TRANSPORT: RefCell<Option<Rc<FakeTransport>>> = const { RefCell::new(None) };
}

/// 統計の画面の依存を組む。API は thread_local の偽の送信の実装を使う。
fn stats_services() -> (RecordServices, AuthServices) {
    let transport = STATS_TRANSPORT.with(|slot| {
        slot.borrow()
            .clone()
            .expect("the transport must be set before mounting")
    });
    let api = ApiClient::new(transport);
    let auth = AuthServices::new(api.clone(), Rc::new(FakePasskeyClient::new()));
    let uploader = PhotoUploader::new(
        RecordsApi::new(api.clone()),
        Rc::new(FakeUploadTransport::new()),
    );
    let records = RecordServices::new(
        api,
        Rc::new(FakeClock {
            now: LocalDateTime::new(2026, 9, 25, 12, 30),
            utc_offset_minutes: 540,
        }),
        Rc::new(FakePhotoPicker { photo: None }),
        Rc::new(FakeImageConverter),
        Rc::new(uploader),
    );
    (records, auth)
}

/// 統計の画面だけを持つテスト用の経路 (ルーターの文脈を用意する)。
#[derive(Routable, Clone, PartialEq, Debug)]
enum TestRoute {
    #[route("/stats", StatsRoute)]
    Stats {},
}

#[component]
fn StatsRoute() -> Element {
    rsx! {
        StatsScreen {}
    }
}

/// 統計の画面を、ルーターの文脈と依存を与えて描く探り。
#[component]
fn StatsScreenProbe() -> Element {
    let (services, auth) = stats_services();
    let _services = use_context_provider(|| services);
    let _auth = use_context_provider(|| auth);
    let _session = use_context_provider(|| Signal::new(SessionStatus::SignedIn));
    let _notice = use_context_provider(|| Signal::new(None::<String>));
    rsx! {
        HistoryProvider {
            history: move |_| Rc::new(MemoryHistory::with_initial_path("/stats")) as Rc<dyn History>,
            Router::<TestRoute> {}
        }
    }
}

/// 入れ物の中の一致を順に返す (NodeList の添字アクセス)。
fn elements(root: &web_sys::Element, selector: &str) -> Vec<web_sys::Element> {
    let list = root
        .query_selector_all(selector)
        .expect("the selector must be valid");
    let mut found = Vec::new();
    for index in 0..list.length() {
        let value = js_sys::Reflect::get(&list, &wasm_bindgen::JsValue::from_f64(f64::from(index)))
            .expect("the index must be readable");
        if let Ok(element) = value.dyn_into::<web_sys::Element>() {
            found.push(element);
        }
    }
    found
}

/// 抽出の区間の集計。
fn brew_periods() -> Vec<BrewPeriod> {
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
        BrewPeriod {
            period: "2026-09-03".to_string(),
            brew_count: 5,
            dose_grams: 75.0,
        },
    ]
}

/// 評価を持つ抽出 (条件が null の 1 件を含む)。
fn brew_ratings() -> Vec<BrewRating> {
    vec![
        BrewRating {
            id: "brew-1".to_string(),
            dose_grams: Some(15.0),
            water_grams: Some(250.0),
            water_temp_c: Some(92.0),
            brew_time_seconds: Some(150),
            rating: 4,
        },
        BrewRating {
            id: "brew-2".to_string(),
            dose_grams: None,
            water_grams: None,
            water_temp_c: None,
            brew_time_seconds: None,
            rating: 5,
        },
        BrewRating {
            id: "brew-3".to_string(),
            dose_grams: Some(18.0),
            water_grams: Some(240.0),
            water_temp_c: Some(88.0),
            brew_time_seconds: Some(180),
            rating: 5,
        },
    ]
}

#[component]
fn BrewChartsProbe() -> Element {
    rsx! {
        BrewStatsCharts { periods: brew_periods() }
    }
}

/// 抽出回数と豆の消費量の棒グラフが、それぞれの系列と 2 つの色で描かれることを検査する。
#[wasm_bindgen_test]
async fn the_brew_charts_draw_the_count_and_dose_series() {
    install_styles();
    set_theme("paper");
    let root = mount(BrewChartsProbe).await;

    assert_eq!(count(&root, "#stats-brew-count-chart rect.count"), 3);
    assert_eq!(count(&root, "#stats-brew-dose-chart rect.grams"), 3);
    assert_eq!(computed(&select(&root, ".count"), "fill"), CHART_COUNT);
    assert_eq!(computed(&select(&root, ".grams"), "fill"), CHART_GRAMS);
    // 値の大きい棒ほど上に高く描く (3 番目の 5 杯が最大)。
    let largest = select(&root, "#stats-brew-count-chart rect.count:nth-of-type(3)");
    let smaller = select(&root, "#stats-brew-count-chart rect.count:nth-of-type(2)");
    let largest_y: f64 = largest
        .get_attribute("y")
        .expect("the bar must have y")
        .parse()
        .expect("y must be a number");
    let smaller_y: f64 = smaller
        .get_attribute("y")
        .expect("the bar must have y")
        .parse()
        .expect("y must be a number");
    assert!(largest_y < smaller_y, "the larger value must be higher");
    // 合計のタイル (抽出回数と豆の消費量) を上に置く。
    assert!(select(&root, ".stat-tile .v").inner_html().contains("10"));
    assert!(select(&root, ".stat-tiles .stat-tile:nth-of-type(2) .v")
        .inner_html()
        .contains("150.5"));
}

#[component]
fn ScatterProbe() -> Element {
    rsx! {
        BrewRatingScatterCharts { ratings: brew_ratings() }
    }
}

/// 散布図 4 つが条件ごとの点を持ち、条件が null の抽出を描かないことを検査する (FR-18)。
#[wasm_bindgen_test]
async fn the_scatter_charts_skip_the_null_conditions() {
    install_styles();
    set_theme("paper");
    let root = mount(ScatterProbe).await;

    assert_eq!(count(&root, ".chart.nb svg"), 4);
    for id in [
        "#stats-rating-dose-chart",
        "#stats-rating-water-chart",
        "#stats-rating-temp-chart",
        "#stats-rating-time-chart",
    ] {
        assert_eq!(count(&root, &format!("{id} circle.dot")), 2, "{id}");
    }
    // 点は chart-grams (crema) で描く。
    assert_eq!(
        computed(
            &select(&root, "#stats-rating-dose-chart circle.dot"),
            "fill"
        ),
        CHART_GRAMS
    );
    // 評価 5 の点は評価 4 の点より上にある (縦軸は 1 から 5)。
    let high = select(&root, "#stats-rating-dose-chart circle.dot:nth-of-type(2)");
    let low = select(&root, "#stats-rating-dose-chart circle.dot:nth-of-type(1)");
    let high_cy: f64 = high
        .get_attribute("cy")
        .expect("the dot must have cy")
        .parse()
        .expect("cy must be a number");
    let low_cy: f64 = low
        .get_attribute("cy")
        .expect("the dot must have cy")
        .parse()
        .expect("cy must be a number");
    assert!(high_cy < low_cy, "rating 5 must be above rating 4");
}

#[component]
fn PurchaseChartsProbe() -> Element {
    rsx! {
        PurchaseStatsCharts {
            purchases: vec![
                PurchasePeriod {
                    period: "2026-09-01".to_string(),
                    price_currency: Some("JPY".to_string()),
                    price_amount: 1200,
                    weight_grams: 200,
                    purchase_count: 1,
                },
                PurchasePeriod {
                    period: "2026-09-02".to_string(),
                    price_currency: Some("JPY".to_string()),
                    price_amount: 800,
                    weight_grams: 150,
                    purchase_count: 1,
                },
                PurchasePeriod {
                    period: "2026-09-01".to_string(),
                    price_currency: None,
                    price_amount: 0,
                    weight_grams: 100,
                    purchase_count: 1,
                },
            ],
        }
    }
}

/// 購入金額と重量の棒グラフが通貨コードごとに分かれることを検査する (FR-18)。
#[wasm_bindgen_test]
async fn the_purchase_charts_are_split_by_currency() {
    install_styles();
    set_theme("paper");
    let root = mount(PurchaseChartsProbe).await;

    assert_eq!(
        count(&root, "#stats-purchase-amount-chart-JPY rect.count"),
        2
    );
    assert_eq!(
        count(&root, "#stats-purchase-weight-chart-JPY rect.grams"),
        2
    );
    // 価格が無い購入は通貨コードが null の組にする。
    assert_eq!(
        count(&root, "#stats-purchase-amount-chart-none rect.count"),
        1
    );
    assert_eq!(
        count(&root, "#stats-purchase-weight-chart-none rect.grams"),
        1
    );
    assert_eq!(
        computed(
            &select(&root, "#stats-purchase-amount-chart-JPY rect.count"),
            "fill"
        ),
        CHART_COUNT
    );
}

#[component]
fn EmptyChartsProbe() -> Element {
    rsx! {
        BrewStatsCharts { periods: Vec::new() }
        PurchaseStatsCharts { purchases: Vec::new() }
        BrewRatingScatterCharts { ratings: Vec::new() }
        RatingHistoryChart { entries: Vec::new() }
    }
}

/// 集計が空の区画に「記録がありません」を出すことを検査する (Charts のガイドライン)。
#[wasm_bindgen_test]
async fn the_empty_charts_show_the_no_records_message() {
    install_styles();
    set_theme("paper");
    set_language(Language::English);
    let root = mount(EmptyChartsProbe).await;

    // 抽出の 2 つ、購入の 2 つ、散布図 4 つ、折れ線 1 つ。
    assert_eq!(count(&root, ".empty"), 9);
    let message = text(Language::English, Key::NoRecords);
    for element in elements(&root, ".empty") {
        assert_eq!(element.inner_html(), message);
    }
}

#[component]
fn RatingHistoryProbe() -> Element {
    rsx! {
        RatingHistoryChart {
            entries: vec![
                RatingHistoryEntry {
                    id: "brew-1".to_string(),
                    brewed_at: "2026-09-01T00:00:00.000Z".to_string(),
                    rating: 2,
                },
                RatingHistoryEntry {
                    id: "brew-2".to_string(),
                    brewed_at: "2026-09-10T00:00:00.000Z".to_string(),
                    rating: 5,
                },
            ],
        }
    }
}

/// 評価の推移の折れ線が chart-grams の線と点で描かれることを検査する (FR-18)。
#[wasm_bindgen_test]
async fn the_rating_history_line_chart_draws_the_line() {
    install_styles();
    set_theme("paper");
    let root = mount(RatingHistoryProbe).await;

    assert_eq!(
        count(&root, "#purchase-rating-history-chart polyline.line"),
        1
    );
    assert_eq!(count(&root, "#purchase-rating-history-chart circle.pt"), 2);
    assert_eq!(
        computed(
            &select(&root, "#purchase-rating-history-chart polyline.line"),
            "stroke"
        ),
        CHART_GRAMS
    );
    // 点は paper-raised の塗りに chart-grams の輪 (docs/design/components/Charts)。
    let dot = select(&root, "#purchase-rating-history-chart circle.pt");
    assert_eq!(computed(&dot, "fill"), PAPER_RAISED);
    assert_eq!(computed(&dot, "stroke"), CHART_GRAMS);
    // 杯数を右端に出す。
    assert!(select(&root, ".chart .h .s").inner_html().contains('2'));
}

#[component]
fn AllChartsProbe() -> Element {
    rsx! {
        BrewStatsCharts { periods: brew_periods() }
        PurchaseStatsCharts {
            purchases: vec![PurchasePeriod {
                period: "2026-09-01".to_string(),
                price_currency: Some("JPY".to_string()),
                price_amount: 1200,
                weight_grams: 200,
                purchase_count: 1,
            }],
        }
        BrewRatingScatterCharts { ratings: brew_ratings() }
        RatingHistoryChart {
            entries: vec![RatingHistoryEntry {
                id: "brew-1".to_string(),
                brewed_at: "2026-09-01T00:00:00.000Z".to_string(),
                rating: 2,
            }],
        }
    }
}

/// 統計の画面の初期状態が、当月の日別の 3 経路を 1 回ずつ呼び、当月の棒グラフを描くことを
/// 検査する (FR-18)。読み込みが止まらないこと (要求が増えないこと) も確かめる。
#[wasm_bindgen_test]
async fn the_stats_screen_shows_the_current_month_daily_charts() {
    install_styles();
    set_theme("paper");
    let transport = Rc::new(FakeTransport::new(vec![
        FakeTransport::response(
            200,
            &json!({
                "brews": [{"period": "2026-09-01", "brew_count": 3, "dose_grams": 45.0}],
            })
            .to_string(),
        ),
        FakeTransport::response(200, &json!({"purchases": []}).to_string()),
        FakeTransport::response(200, &json!({"brew_ratings": []}).to_string()),
    ]));
    STATS_TRANSPORT.with(|slot| *slot.borrow_mut() = Some(transport));
    let root = mount(StatsScreenProbe).await;
    for _ in 0..10 {
        tick().await;
    }

    // 当月 1 日から当日までを日別で、抽出、購入、評価の順に 1 回ずつ呼ぶ。
    let requests = STATS_TRANSPORT.with(|slot| {
        slot.borrow()
            .as_ref()
            .expect("the transport must be set")
            .requests()
    });
    assert_eq!(requests.len(), 3, "{requests:?}");
    assert_eq!(
        requests[0].path,
        "/api/stats/brews?granularity=day&start=2026-09-01&end=2026-09-25&utc_offset_minutes=540"
    );
    assert_eq!(
        requests[1].path,
        "/api/stats/purchases?granularity=day&start=2026-09-01&end=2026-09-25"
    );
    assert_eq!(
        requests[2].path,
        "/api/stats/brew-ratings?start=2026-09-01&end=2026-09-25&utc_offset_minutes=540"
    );

    // 当月の日別の棒グラフと、選択中の期間のチップが出る。
    assert_eq!(count(&root, "#stats-brew-count-chart rect.count"), 1);
    assert_eq!(count(&root, "#stats-brew-dose-chart rect.grams"), 1);
    assert_eq!(count(&root, ".chips .chip.on"), 1);
    assert_eq!(
        select(&root, ".chips .chip.on").inner_html(),
        text(Language::English, Key::StatsPeriodCurrentMonth)
    );
}

/// グラフの色が chart-count と chart-grams の 2 つだけであることを検査する
/// (docs/design/components/Charts。折れ線の点の塗りだけは paper-raised)。
#[wasm_bindgen_test]
async fn the_charts_use_only_the_two_chart_colors() {
    install_styles();
    set_theme("paper");
    let root = mount(AllChartsProbe).await;

    let shapes = elements(
        &root,
        ".chart svg rect, .chart svg circle, .chart svg polyline",
    );
    assert!(!shapes.is_empty(), "the charts must draw shapes");
    for shape in shapes {
        let fill = computed(&shape, "fill");
        assert!(
            matches!(
                fill.as_str(),
                CHART_COUNT | CHART_GRAMS | PAPER_RAISED | "none"
            ),
            "unexpected chart fill: {fill}"
        );
        let stroke = computed(&shape, "stroke");
        assert!(
            matches!(stroke.as_str(), CHART_GRAMS | "none"),
            "unexpected chart stroke: {stroke}"
        );
    }
}

/// 期間のチップを押すと、選んだ期間で読み直すことを検査する (0042 のレビューの指摘)。
#[wasm_bindgen_test]
async fn the_period_chip_reloads_with_the_selected_period() {
    install_styles();
    set_theme("paper");
    let transport = Rc::new(FakeTransport::new(vec![
        // 初期の当月の 3 経路。
        FakeTransport::response(
            200,
            &json!({
                "brews": [{"period": "2026-09-01", "brew_count": 3, "dose_grams": 45.0}],
            })
            .to_string(),
        ),
        FakeTransport::response(200, &json!({"purchases": []}).to_string()),
        FakeTransport::response(200, &json!({"brew_ratings": []}).to_string()),
        // 3 か月への切り替えの 3 経路。
        FakeTransport::response(
            200,
            &json!({
                "brews": [{"period": "2026-07", "brew_count": 8, "dose_grams": 120.0}],
            })
            .to_string(),
        ),
        FakeTransport::response(200, &json!({"purchases": []}).to_string()),
        FakeTransport::response(200, &json!({"brew_ratings": []}).to_string()),
    ]));
    STATS_TRANSPORT.with(|slot| *slot.borrow_mut() = Some(transport));
    let root = mount(StatsScreenProbe).await;
    for _ in 0..10 {
        tick().await;
    }

    // 期間のチップは 6 つ (当月、3 か月、6 か月、12 か月、全期間、任意)。
    let chips = elements(&root, ".chips .chip");
    assert_eq!(chips.len(), 6, "the preset chips must be shown");
    // 2 番目 (3 か月) を押す。
    chips[1]
        .dyn_ref::<web_sys::HtmlElement>()
        .expect("the chip must be clickable")
        .click();
    for _ in 0..10 {
        tick().await;
    }

    let requests = STATS_TRANSPORT.with(|slot| {
        slot.borrow()
            .as_ref()
            .expect("the transport must be set")
            .requests()
    });
    assert_eq!(requests.len(), 6, "{requests:?}");
    // 3 か月 (87 日) は月別になる (62 日を超えるため)。
    assert_eq!(
        requests[3].path,
        "/api/stats/brews?granularity=month&start=2026-07-01&end=2026-09-25&utc_offset_minutes=540"
    );
    assert_eq!(
        requests[4].path,
        "/api/stats/purchases?granularity=month&start=2026-07-01&end=2026-09-25"
    );
    assert_eq!(
        requests[5].path,
        "/api/stats/brew-ratings?start=2026-07-01&end=2026-09-25&utc_offset_minutes=540"
    );

    // 選択中のチップが 1 つだけになり、3 か月になる。
    assert_eq!(count(&root, ".chips .chip.on"), 1);
    assert_eq!(
        select(&root, ".chips .chip.on").inner_html(),
        text(Language::English, Key::StatsPeriodThreeMonths)
    );
}

/// 任意の期間の開始日と終了日が date input で描かれ、値が `YYYY-MM-DD` になることを検査する
/// (完了条件 1、3)。
///
/// 表示の形式 (ブラウザの言語による yyyy/mm/dd などの見え方) は検査しない。内部の値の形式
/// だけを確かめる。
#[wasm_bindgen_test]
async fn the_custom_period_dates_use_the_native_date_inputs() {
    install_styles();
    set_theme("paper");
    let transport = Rc::new(FakeTransport::new(vec![
        FakeTransport::response(200, &json!({"brews": []}).to_string()),
        FakeTransport::response(200, &json!({"purchases": []}).to_string()),
        FakeTransport::response(200, &json!({"brew_ratings": []}).to_string()),
    ]));
    STATS_TRANSPORT.with(|slot| *slot.borrow_mut() = Some(transport));
    let root = mount(StatsScreenProbe).await;
    for _ in 0..10 {
        tick().await;
    }

    // 「任意」を選ぶまで入力欄は出ない (FR-18)。
    assert_eq!(count(&root, ".stats-custom"), 0);
    let chips = elements(&root, ".chips .chip");
    assert_eq!(chips.len(), 6, "the preset chips must be shown");
    chips[5]
        .dyn_ref::<web_sys::HtmlElement>()
        .expect("the chip must be clickable")
        .click();
    for _ in 0..5 {
        tick().await;
    }

    // 開始日は当月の 1 日、終了日は当日 (端末のタイムゾーン。FR-18)。
    assert_eq!(count(&root, ".stats-custom input[type='date']"), 2);
    let dates = elements(&root, ".stats-custom input[type='date']");
    assert_eq!(
        dates[0]
            .unchecked_ref::<web_sys::HtmlInputElement>()
            .value(),
        "2026-09-01"
    );
    assert_eq!(
        dates[1]
            .unchecked_ref::<web_sys::HtmlInputElement>()
            .value(),
        "2026-09-25"
    );
    // 形式の案内は help に出し、placeholder と icon は使わない (0049 の原本の更新による)。
    assert_eq!(count(&root, ".stats-custom .field .help"), 2);
    assert!(dates.iter().all(|date| date
        .get_attribute("placeholder")
        .unwrap_or_default()
        .is_empty()));
    assert_eq!(count(&root, ".stats-custom input[type='date'] ~ .icon"), 0);
}
