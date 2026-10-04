//! `stats` の単体テスト。
//!
//! 組み立てた SQL が利用者 ID の条件を必ず含むこと、値がプレースホルダで
//! 渡ること、期間の端を UTC の瞬間に直すこと、入力の検証の誤りが 400 になることを確認する。
//!
//! 組み立てる関数ごとの SQL は [`queries`]、条件の付け忘れの検出は [`conditions`] が検査する。

use brew_book_core::stats::{self, Granularity, Period, StatsError};

const USER_ID: &str = "9f8f1f2e-6b1a-4a3c-8d0e-1b2c3d4e5f60";
const PURCHASE_ID: &str = "0d2b6f5e-3a4c-4a7b-9c8d-7e6f5a4b3c2d";

mod validation {
    use super::*;

    #[test]
    fn the_granularity_accepts_day_and_month_only() {
        assert_eq!(stats::parse_granularity(Some("day")), Ok(Granularity::Day));
        assert_eq!(
            stats::parse_granularity(Some("month")),
            Ok(Granularity::Month)
        );
        // 必須のため、省略は拒否する (FR-18)。
        for text in [
            None,
            Some(""),
            Some("Day"),
            Some("MONTH"),
            Some("days"),
            Some("week"),
        ] {
            assert_eq!(
                stats::parse_granularity(text),
                Err(StatsError::Granularity),
                "the granularity must be rejected: {text:?}"
            );
        }
        assert_eq!(StatsError::Granularity.code().status(), 400);
        assert!(!StatsError::Granularity.message().is_empty());
    }

    #[test]
    fn the_offset_accepts_the_range_from_minus_840_to_840() {
        assert_eq!(stats::parse_offset_minutes(Some("540")), Ok(540));
        assert_eq!(stats::parse_offset_minutes(Some("0")), Ok(0));
        assert_eq!(stats::parse_offset_minutes(Some("-300")), Ok(-300));
        // 端は受け付ける。
        assert_eq!(stats::parse_offset_minutes(Some("840")), Ok(840));
        assert_eq!(stats::parse_offset_minutes(Some("-840")), Ok(-840));
        // 符号付きの表記も整数として受け付ける。
        assert_eq!(stats::parse_offset_minutes(Some("+540")), Ok(540));
        for text in [
            None,
            Some(""),
            Some(" "),
            Some("abc"),
            Some("540.0"),
            Some("540分"),
            Some("1e3"),
            Some("841"),
            Some("-841"),
            Some("2147483648"),
            // 利用者の入力を SQL に連結しないため、数値でない値はここで拒否する (ADR-0006)。
            Some("0 minutes) OR 1 = 1 --"),
        ] {
            assert_eq!(
                stats::parse_offset_minutes(text),
                Err(StatsError::Offset),
                "the offset must be rejected: {text:?}"
            );
        }
        assert_eq!(StatsError::Offset.code().status(), 400);
        assert!(!StatsError::Offset.message().is_empty());
    }

    #[test]
    fn the_offset_modifier_is_built_from_the_integer() {
        assert_eq!(stats::offset_modifier(540), "+540 minutes");
        assert_eq!(stats::offset_modifier(-300), "-300 minutes");
        assert_eq!(stats::offset_modifier(0), "+0 minutes");
        assert_eq!(stats::offset_modifier(840), "+840 minutes");
        assert_eq!(stats::offset_modifier(-840), "-840 minutes");
    }

    #[test]
    fn the_period_accepts_existing_dates_and_omittable_ends() {
        // 両方を省略できる (全期間)。省略した端には条件を付けない (FR-18)。
        assert_eq!(stats::parse_period(None, None), Ok(Period::default()));
        assert_eq!(
            stats::parse_period(Some("2026-09-21"), None),
            Ok(Period {
                start: Some("2026-09-21".to_owned()),
                end: None,
            })
        );
        assert_eq!(
            stats::parse_period(None, Some("2026-09-21")),
            Ok(Period {
                start: None,
                end: Some("2026-09-21".to_owned()),
            })
        );
        // 同じ日は受け付ける (開始日が終了日より後だけを拒否する)。
        assert_eq!(
            stats::parse_period(Some("2026-09-21"), Some("2026-09-21")),
            Ok(Period {
                start: Some("2026-09-21".to_owned()),
                end: Some("2026-09-21".to_owned()),
            })
        );
        // うるう年の 2 月 29 日は実在する日付として受け付ける。
        assert!(stats::parse_period(Some("2028-02-29"), Some("2028-03-01")).is_ok());
    }

    #[test]
    fn the_period_rejects_a_reversed_range() {
        assert_eq!(
            stats::parse_period(Some("2026-09-22"), Some("2026-09-21")),
            Err(StatsError::ReversedPeriod)
        );
        assert_eq!(StatsError::ReversedPeriod.code().status(), 400);
        assert!(!StatsError::ReversedPeriod.message().is_empty());
    }

    #[test]
    fn the_period_rejects_values_that_are_not_existing_dates() {
        for text in [
            // 形式が違う。
            "",
            " ",
            "2026-9-21",
            "2026-09-2",
            "20260921",
            "2026/09/21",
            " 2026-09-21",
            "2026-09-21 ",
            "2026-09-21T00:00:00.000Z",
            // 実在しない日付 (うるう年を考慮する)。
            "2026-02-30",
            "2026-13-01",
            "2026-00-10",
            "2026-09-00",
            "2026-09-31",
            "2027-02-29",
        ] {
            assert_eq!(
                stats::parse_period(Some(text), None),
                Err(StatsError::Date),
                "the start date must be rejected: {text:?}"
            );
            assert_eq!(
                stats::parse_period(None, Some(text)),
                Err(StatsError::Date),
                "the end date must be rejected: {text:?}"
            );
        }
        assert_eq!(StatsError::Date.code().status(), 400);
        assert!(!StatsError::Date.message().is_empty());
    }
}

mod queries {
    use super::*;
    use brew_book_core::query::Value;

    /// 日別の抽出回数と豆の消費量の SQL。
    #[test]
    fn the_brew_stats_query_groups_by_the_local_day() {
        let period = Period {
            start: Some("2026-09-21".to_owned()),
            end: Some("2026-09-21".to_owned()),
        };
        let statement = stats::brews_stats(USER_ID, Granularity::Day, 540, &period).unwrap();
        assert_eq!(
            statement.sql,
            "SELECT strftime('%Y-%m-%d', b.brewed_at, '+540 minutes') AS period, \
             COUNT(*) AS brew_count, COALESCE(ROUND(SUM(b.dose_grams), 1), 0) AS dose_grams \
             FROM brews AS b WHERE b.user_id = ? \
             AND b.brewed_at >= ? AND b.brewed_at < ? \
             GROUP BY strftime('%Y-%m-%d', b.brewed_at, '+540 minutes') ORDER BY period ASC"
        );
        // 開始日はその日の 00:00、終了日は翌日の 00:00 を UTC の瞬間に直す (FR-18)。
        assert_eq!(
            statement.params,
            vec![
                Value::Text(USER_ID.to_owned()),
                Value::Text("2026-09-20T15:00:00.000Z".to_owned()),
                Value::Text("2026-09-21T15:00:00.000Z".to_owned()),
            ]
        );
    }

    /// 月別の抽出回数と豆の消費量の SQL。オフセットが負でも修飾子を組み立てる。
    #[test]
    fn the_brew_stats_query_groups_by_the_local_month() {
        let statement =
            stats::brews_stats(USER_ID, Granularity::Month, -300, &Period::default()).unwrap();
        assert_eq!(
            statement.sql,
            "SELECT strftime('%Y-%m', b.brewed_at, '-300 minutes') AS period, \
             COUNT(*) AS brew_count, COALESCE(ROUND(SUM(b.dose_grams), 1), 0) AS dose_grams \
             FROM brews AS b WHERE b.user_id = ? \
             GROUP BY strftime('%Y-%m', b.brewed_at, '-300 minutes') ORDER BY period ASC"
        );
        assert_eq!(statement.params, vec![Value::Text(USER_ID.to_owned())]);
    }

    /// 片方の端だけを指定したときは、その端の条件だけを付ける。
    #[test]
    fn the_brew_stats_query_adds_only_the_given_ends() {
        let start_only = stats::brews_stats(
            USER_ID,
            Granularity::Day,
            0,
            &Period {
                start: Some("2026-09-21".to_owned()),
                end: None,
            },
        )
        .unwrap();
        assert!(start_only.sql.contains("AND b.brewed_at >= ?"));
        assert!(!start_only.sql.contains("b.brewed_at < ?"));
        assert_eq!(
            start_only.params,
            vec![
                Value::Text(USER_ID.to_owned()),
                Value::Text("2026-09-21T00:00:00.000Z".to_owned()),
            ]
        );

        let end_only = stats::brews_stats(
            USER_ID,
            Granularity::Day,
            0,
            &Period {
                start: None,
                end: Some("2026-09-21".to_owned()),
            },
        )
        .unwrap();
        assert!(!end_only.sql.contains("b.brewed_at >= ?"));
        assert!(end_only.sql.contains("AND b.brewed_at < ?"));
        assert_eq!(
            end_only.params,
            vec![
                Value::Text(USER_ID.to_owned()),
                Value::Text("2026-09-22T00:00:00.000Z".to_owned()),
            ]
        );
    }

    /// 評価の散布図の SQL。評価が NULL の抽出を除く。
    #[test]
    fn the_brew_ratings_query_keeps_only_the_rated_brews() {
        let period = Period {
            start: Some("2026-09-21".to_owned()),
            end: Some("2026-09-21".to_owned()),
        };
        let statement = stats::brew_ratings(USER_ID, -300, &period).unwrap();
        assert_eq!(
            statement.sql,
            "SELECT b.id, b.dose_grams, b.water_grams, b.water_temp_c, b.brew_time_seconds, \
             b.rating FROM brews AS b WHERE b.user_id = ? \
             AND b.rating IS NOT NULL AND b.brewed_at >= ? AND b.brewed_at < ? \
             ORDER BY b.brewed_at ASC, b.id ASC"
        );
        assert_eq!(
            statement.params,
            vec![
                Value::Text(USER_ID.to_owned()),
                Value::Text("2026-09-21T05:00:00.000Z".to_owned()),
                Value::Text("2026-09-22T05:00:00.000Z".to_owned()),
            ]
        );
    }

    /// 購入金額と重量の SQL。通貨コードの昇順 (NULL が先頭) で並べる。
    #[test]
    fn the_purchase_stats_query_groups_by_the_period_and_the_currency() {
        let period = Period {
            start: Some("2026-09-21".to_owned()),
            end: Some("2026-09-21".to_owned()),
        };
        let statement = stats::purchases_stats(USER_ID, Granularity::Day, &period);
        assert_eq!(
            statement.sql,
            "SELECT strftime('%Y-%m-%d', p.purchased_on) AS period, p.price_currency, \
             COALESCE(SUM(p.price_amount), 0) AS price_amount, \
             COALESCE(SUM(p.weight_grams), 0) AS weight_grams, COUNT(*) AS purchase_count \
             FROM purchases AS p WHERE p.user_id = ? \
             AND p.purchased_on >= ? AND p.purchased_on <= ? \
             GROUP BY strftime('%Y-%m-%d', p.purchased_on), p.price_currency \
             ORDER BY period ASC, p.price_currency ASC"
        );
        // 購入日はタイムゾーンを持たないため、日付をそのまま絞る (FR-18)。
        assert_eq!(
            statement.params,
            vec![
                Value::Text(USER_ID.to_owned()),
                Value::Text("2026-09-21".to_owned()),
                Value::Text("2026-09-21".to_owned()),
            ]
        );
    }

    /// 月別の購入金額と重量の SQL。
    #[test]
    fn the_purchase_stats_query_groups_by_the_month() {
        let statement = stats::purchases_stats(USER_ID, Granularity::Month, &Period::default());
        assert_eq!(
            statement.sql,
            "SELECT strftime('%Y-%m', p.purchased_on) AS period, p.price_currency, \
             COALESCE(SUM(p.price_amount), 0) AS price_amount, \
             COALESCE(SUM(p.weight_grams), 0) AS weight_grams, COUNT(*) AS purchase_count \
             FROM purchases AS p WHERE p.user_id = ? \
             GROUP BY strftime('%Y-%m', p.purchased_on), p.price_currency \
             ORDER BY period ASC, p.price_currency ASC"
        );
        assert_eq!(statement.params, vec![Value::Text(USER_ID.to_owned())]);
    }

    /// 購入ごとの評価の推移の SQL。期間で絞らない。
    #[test]
    fn the_rating_history_query_is_ordered_by_the_brew_timestamp() {
        let statement = stats::rating_history(USER_ID, PURCHASE_ID);
        assert_eq!(
            statement.sql,
            "SELECT b.id, b.brewed_at, b.rating FROM brews AS b WHERE b.user_id = ? \
             AND b.purchase_id = ? AND b.rating IS NOT NULL \
             ORDER BY b.brewed_at ASC, b.id ASC"
        );
        assert_eq!(
            statement.params,
            vec![
                Value::Text(USER_ID.to_owned()),
                Value::Text(PURCHASE_ID.to_owned()),
            ]
        );
        assert!(!statement.sql.contains("brewed_at >="));
    }

    /// 扱える日時の範囲の外になる端は、日付の誤りにする。
    #[test]
    fn an_end_outside_the_supported_range_is_rejected_as_a_date() {
        let period = Period {
            start: Some("0000-01-01".to_owned()),
            end: None,
        };
        // 0000-01-01 の 00:00 は扱える最も古い時刻のため、正のオフセットを引くと範囲の外になる。
        assert_eq!(
            stats::brews_stats(USER_ID, Granularity::Day, 840, &period),
            Err(StatsError::Date)
        );
        assert_eq!(
            stats::brew_ratings(USER_ID, 840, &period),
            Err(StatsError::Date)
        );
        let period = Period {
            start: None,
            end: Some("9999-12-31".to_owned()),
        };
        // 終了日の翌日は扱える最も新しい時刻の翌日になるため、負のオフセットでは範囲の外になる。
        assert_eq!(
            stats::brews_stats(USER_ID, Granularity::Day, -840, &period),
            Err(StatsError::Date)
        );
    }
}

mod conditions {
    use super::*;
    use brew_book_core::query::{Statement, Value};

    /// 統計の文の一覧。
    fn every_statement() -> Vec<(&'static str, Statement)> {
        let period = Period {
            start: Some("2026-09-21".to_owned()),
            end: Some("2026-09-22".to_owned()),
        };
        let mut statements = vec![
            (
                "brews stats (day)",
                stats::brews_stats(USER_ID, Granularity::Day, 540, &period).unwrap(),
            ),
            (
                "brews stats (month, all)",
                stats::brews_stats(USER_ID, Granularity::Month, -300, &Period::default()).unwrap(),
            ),
            (
                "brew ratings (period)",
                stats::brew_ratings(USER_ID, 540, &period).unwrap(),
            ),
            (
                "brew ratings (all)",
                stats::brew_ratings(USER_ID, 0, &Period::default()).unwrap(),
            ),
            (
                "purchases stats (day)",
                stats::purchases_stats(USER_ID, Granularity::Day, &period),
            ),
            (
                "purchases stats (month, all)",
                stats::purchases_stats(USER_ID, Granularity::Month, &Period::default()),
            ),
            (
                "rating history",
                stats::rating_history(USER_ID, PURCHASE_ID),
            ),
        ];
        statements.extend([
            ("brews stats (only start)", {
                let period = Period {
                    start: Some("2026-09-21".to_owned()),
                    end: None,
                };
                stats::brews_stats(USER_ID, Granularity::Day, 0, &period).unwrap()
            }),
            ("purchases stats (only end)", {
                let period = Period {
                    start: None,
                    end: Some("2026-09-22".to_owned()),
                };
                stats::purchases_stats(USER_ID, Granularity::Day, &period)
            }),
        ]);
        statements
    }

    /// 文が利用者の条件を保つかを確かめる。誤りは理由を返す。
    fn missing_condition(name: &str, statement: &Statement) -> Option<String> {
        if !statement.sql.contains("user_id = ?") {
            return Some(format!("the user filter is missing in {name}"));
        }
        if !statement.params.contains(&Value::Text(USER_ID.to_owned())) {
            return Some(format!("the user id is missing in {name}"));
        }
        None
    }

    #[test]
    fn every_stats_query_keeps_the_user_condition() {
        for (name, statement) in every_statement() {
            assert_eq!(missing_condition(name, &statement), None);
            let placeholders = statement.sql.matches('?').count();
            assert_eq!(
                placeholders,
                statement.params.len(),
                "every placeholder must have a value in {name}: {}",
                statement.sql
            );
            // 利用者の入力の文字列は SQL に連結しない (ADR-0006)。
            assert!(
                !statement.sql.contains(USER_ID) && !statement.sql.contains(PURCHASE_ID),
                "the user input must not appear in the SQL of {name}: {}",
                statement.sql
            );
        }
    }

    #[test]
    fn the_checker_detects_an_omitted_user_condition() {
        let statement = Statement {
            sql: "SELECT id FROM brews WHERE id = ?".to_owned(),
            params: Vec::new(),
        };
        assert_eq!(
            missing_condition("test", &statement),
            Some("the user filter is missing in test".to_owned())
        );
    }

    #[test]
    fn the_checker_accepts_a_query_with_the_user_condition() {
        let statement = Statement {
            sql: "SELECT id FROM brews WHERE user_id = ?".to_owned(),
            params: vec![Value::Text(USER_ID.to_owned())],
        };
        assert_eq!(missing_condition("test", &statement), None);
    }
}
