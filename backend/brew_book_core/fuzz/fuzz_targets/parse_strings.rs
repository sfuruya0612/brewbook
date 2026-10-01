#![no_main]

//! 入力の文字列のパーサの Fuzzing ターゲット (issue 0028)。
//!
//! 任意の UTF-8 の文字列を、入力の文字列を読むパーサに与えて panic しないことを検証する。
//! 成功側の経路 (形式に合う入力でなければ届かない分岐) には、代表的な入力 (`SEEDS`) も与える。
//! CI では実行せず、手元で実行する (cargo fuzz は nightly を要する。ADR-0009 の版固定と
//! 二重管理を避ける)。実行は `backend/brew_book_core` で
//! `cargo +nightly fuzz run parse_strings -- -max_total_time=60` とする (README の Fuzzing)。

use libfuzzer_sys::fuzz_target;

/// 紐づけ前の写真のキーの検査に使う固定の利用者 ID。
const USER_ID: &str = "00000000-0000-4000-8000-000000000000";

/// 形式に合う入力でなければ届かない分岐のための代表的な入力。
const SEEDS: &[&str] = &[
    // datetime と日付の形式 (うるう年でない 2 月 29 日と月末を含む)。
    "2026-09-26T00:00:00.000Z",
    "2026-09-26",
    "2026-02-29",
    "2026-12-31",
    // 統計と一覧のクエリの成功側。
    "day",
    "month",
    "0",
    "50",
    "540",
    "-840",
    "true",
    "false",
    // サジェストの項目名。
    "producer",
    "origin",
    "region",
    "process",
    "variety",
    "roast",
    "method",
    "grind_setting",
    // UUID そのもの (ids::uuid_bytes の成功側)。
    USER_ID,
    // 期間 (parse_period の成功側と、開始と終了が逆転した側)。
    "2026-09-012026-09-30",
    "2026-09-302026-09-01",
    // カーソルの base64url (cursor::CursorKey::decode の成功側)。
    "eyJvbiI6IjIwMjYtMDktMjYiLCJpZCI6ImlkLTEifQ",
    "eyJhdCI6IjIwMjYtMDktMjZUMDA6MDA6MDAuMDAwWiIsImlkIjoiaWQtMSJ9",
    // clientDataJSON の base64url (client_data_challenge の成功側)。
    "eyJ0eXBlIjoid2ViYXV0aG4uZ2V0IiwiY2hhbGxlbmdlIjoiWTJoaGJHeGxibWRsIiwib3JpZ2luIjoiaHR0cHM6Ly9leGFtcGxlLm9yZyJ9",
    // 写真の紐づけ前のキーと Cookie ヘッダ。
    "pending/00000000-0000-4000-8000-000000000000/00000000-0000-4000-8000-000000000000.jpg",
    "session=token",
    // 名前、通貨、数の形式。
    "name",
    "JPY",
    "1.5",
    // 推測の応答 (suggestion::parse_response の成功側)。
    r#"{"product": {"name": "エチオピア イルガチェフェ", "origin": "エチオピア", "flavor_notes": ["フローラル"]}, "roast": "中煎り", "roast_date": "2026-09-20", "price_amount": 1200, "weight_grams": 200}"#,
    "```json\n{\"roast\": null}\n```",
    // AI バインディングの応答の形 (suggestion::output_text と parse_response の成功側)。
    r#"{"response": "{\"roast\": \"中煎り\", \"roast_date\": \"2026-09-20\"}"}"#,
    r#"{"choices": [{"index": 0, "message": {"role": "assistant", "content": "{\"roast\": \"中煎り\"}"}}], "usage": {"total_tokens": 100}}"#,
    // 経路のパターンとパス。
    "/api/shops/:id",
    "/api/shops/00000000-0000-4000-8000-000000000000",
];

/// 1 つの JSON の値を、AI バインディングの応答からの出力の取り出しと解析に与える (FR-19)。
fn exercise_model_result(value: &serde_json::Value) {
    let _ = brew_book_core::suggestion::parse_response(
        &brew_book_core::suggestion::output_text(value),
    );
}

/// 1 つの文字列を、入力の文字列を読むパーサの全てに与える。
fn exercise(text: &str) {
    let _ = brew_book_core::cursor::parse_page_size(Some(text));
    let _ = brew_book_core::datetime::parse_epoch_millis(text);
    let _ = brew_book_core::datetime::is_valid_date(text);
    let _ = brew_book_core::ids::uuid_bytes(text);
    let _ = brew_book_core::query::parse_include_archived(Some(text));
    let _ = brew_book_core::query::parse_suggestion_field(text);
    let _ = brew_book_core::stats::parse_granularity(Some(text));
    let _ = brew_book_core::stats::parse_offset_minutes(Some(text));

    let _ = brew_book_core::webauthn::client_data_challenge(text);
    let _ = brew_book_core::cursor::CursorKey::decode(text);
    let _ = brew_book_core::auth::session_token(text);
    let _ = brew_book_core::auth::validate_passkey_name(text);

    // 利用者 ID は固定する (`pending/<利用者 ID>/...` の検査を通すため)。
    let _ = brew_book_core::photo::parse_pending_key(USER_ID, text);
    let _ = brew_book_core::routes::pattern_matches(text, text);
    // メソッドは固定する (同じ文字列では経路の一致に届かないため)。
    let _ = brew_book_core::routes::match_route(brew_book_core::routes::ROUTES, "GET", text);
    let _ = brew_book_core::records::validate_name(text);
    let _ = brew_book_core::records::validate_day(text);
    let _ = brew_book_core::records::validate_timestamp(text);
    let _ = brew_book_core::records::validate_currency(text);
    let _ = brew_book_core::records::validate_decimal(text);
    let names = [text.to_owned()];
    let _ = brew_book_core::records::validate_flavor_notes(&names);

    // 写真からの推測の応答の解析 (FR-19)。
    let _ = brew_book_core::suggestion::parse_response(text);
    // AI バインディングの応答の形の文字列は、JSON の値としても与える (FR-19)。
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
        exercise_model_result(&value);
    }

    // start と end は独立した入力にする (同じ文字列を渡すと期間の逆転の分岐に届かない)。
    // 分割の位置は文字の境界に合わせる (バイト列の中間は文字の途中になりうる)。
    let half = text.len() / 2;
    let boundary = (0..=half)
        .rev()
        .find(|&index| text.is_char_boundary(index))
        .unwrap_or(0);
    let (start, end) = text.split_at(boundary);
    let _ = brew_book_core::stats::parse_period(Some(start), Some(end));
}

fuzz_target!(|data: &[u8]| {
    // 任意のバイト列を JSON の値として読めたときは、AI バインディングの応答からの出力の
    // 取り出しと解析にも与える (FR-19。実経路は `parse_response(output_text(v))`)。
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(data) {
        exercise_model_result(&value);
    }
    if let Ok(text) = std::str::from_utf8(data) {
        exercise(text);

        // 任意の入力では成功側に届きにくいパーサのために、代表的な入力を与える。
        // 応答の形の文字列は `exercise` が JSON の値としても与える。
        for seed in SEEDS {
            exercise(seed);
        }
    }
});
