//! `records::chart` の単体テスト (0042)。
//!
//! 境界と代表の値を固定する。一般の性質は PBT (`pbt/tests/prop_chart.rs`) が担う。

use brew_book_frontend::records::chart::{
    axis_max, axis_ticks, bar_rect, bar_slots, format_axis, is_daily_key, label_indices, line_x,
    period_label, rating_y, scatter_x, scatter_x_range, tick_y, LINE_LEFT, LINE_RIGHT, PLOT_BOTTOM,
    PLOT_RIGHT, PLOT_TOP, SCATTER_LEFT,
};

#[test]
fn the_axis_max_covers_the_values() {
    assert_eq!(axis_max(2.0), 2.2);
    // 値が無いか 0 以下のときも軸を潰さない。
    assert_eq!(axis_max(0.0), 1.0);
    assert_eq!(axis_max(-5.0), 1.0);
}

#[test]
fn the_axis_ticks_are_the_zero_the_half_and_the_max() {
    assert_eq!(axis_ticks(2.0), [0.0, 1.1, 2.2]);
    assert_eq!(axis_ticks(0.0), [0.0, 0.5, 1.0]);
}

#[test]
fn the_tick_positions_go_from_the_axis_to_the_top() {
    assert_eq!(tick_y(0.0), PLOT_BOTTOM);
    assert_eq!(tick_y(1.0), PLOT_TOP);
    assert_eq!(tick_y(0.5), 50.0);
    // 範囲の外は丸める。
    assert_eq!(tick_y(-1.0), PLOT_BOTTOM);
    assert_eq!(tick_y(2.0), PLOT_TOP);
}

#[test]
fn the_bars_are_placed_left_to_right_within_the_plot() {
    // デザインのプレビューと同じ 14 区間 (幅 19、間隔 22)。
    let slots = bar_slots(14);
    assert_eq!(slots.len(), 14);
    assert_eq!(slots[0], (29.5, 19.0));
    assert_eq!(slots[1], (51.5, 19.0));
    assert_eq!(slots[13], (315.5, 19.0));
    assert!(bar_slots(0).is_empty());
}

#[test]
fn the_bar_height_follows_the_value() {
    // 最大の 1.1 倍を軸の上限にする (Flutter と同じ)。
    assert_eq!(bar_rect(2.2, 2.0), (PLOT_TOP, PLOT_BOTTOM - PLOT_TOP));
    assert_eq!(bar_rect(1.1, 2.0), (50.0, 42.0));
    assert_eq!(bar_rect(0.0, 2.0), (PLOT_BOTTOM, 0.0));
    // 軸の上限を超える値も枠に収める。
    assert_eq!(bar_rect(10.0, 2.0), (PLOT_TOP, PLOT_BOTTOM - PLOT_TOP));
}

#[test]
fn the_axis_values_are_formatted_with_one_decimal() {
    assert_eq!(format_axis(0.0), "0");
    assert_eq!(format_axis(5.0), "5");
    assert_eq!(format_axis(2.2), "2.2");
    assert_eq!(format_axis(16.5), "16.5");
}

#[test]
fn the_daily_key_is_ten_characters() {
    assert!(is_daily_key("2026-09-01"));
    assert!(!is_daily_key("2026-09"));
    assert!(!is_daily_key(""));
}

#[test]
fn the_period_label_is_the_month_and_day() {
    assert_eq!(period_label("2026-09-01"), "09-01");
    // 月別のキーはそのまま出す。
    assert_eq!(period_label("2026-09"), "2026-09");
    // ISO 8601 の日時は日付の部分だけを出す (折れ線の横軸)。
    assert_eq!(period_label("2026-09-20T14:59:59.999Z"), "09-20");
}

#[test]
fn the_label_indices_are_the_first_the_middle_and_the_last() {
    assert!(label_indices(0).is_empty());
    assert_eq!(label_indices(1), vec![0]);
    assert_eq!(label_indices(3), vec![0, 1, 2]);
    assert_eq!(label_indices(4), vec![0, 2, 3]);
    assert_eq!(label_indices(14), vec![0, 7, 13]);
}

#[test]
fn the_scatter_range_is_widened_when_the_values_are_equal() {
    assert_eq!(scatter_x_range(2.0, 2.0), (1.0, 3.0));
    assert_eq!(scatter_x_range(1.0, 5.0), (1.0, 5.0));
}

#[test]
fn the_scatter_x_spans_the_plot() {
    assert_eq!(scatter_x(12.0, 12.0, 20.0), SCATTER_LEFT);
    assert_eq!(scatter_x(20.0, 12.0, 20.0), PLOT_RIGHT);
    assert_eq!(scatter_x(16.0, 12.0, 20.0), 181.0);
}

#[test]
fn the_rating_y_is_one_at_the_axis_and_five_at_the_top() {
    assert_eq!(rating_y(1.0), PLOT_BOTTOM);
    assert_eq!(rating_y(5.0), PLOT_TOP);
    assert_eq!(rating_y(3.0), 50.0);
}

#[test]
fn the_line_x_spans_the_line() {
    assert_eq!(line_x(0, 6), LINE_LEFT);
    assert_eq!(line_x(5, 6), LINE_RIGHT);
    // 1 点のときは中央に置く。
    assert_eq!(line_x(0, 1), (LINE_LEFT + LINE_RIGHT) / 2.0);
}

/// 壊れた応答のキーでも panic しない (0042 のレビューの指摘)。
#[test]
fn a_broken_key_is_not_sliced_in_the_middle_of_a_character() {
    // 10 バイトだが日付ではない (5 バイト目が文字の途中になる)。
    let broken = "aaaa日aaa";
    assert_eq!(broken.len(), 10);
    assert!(!is_daily_key(broken));
    assert_eq!(period_label(broken), broken);

    // 24 バイトで 10 バイト目が T だが日付ではない。
    let broken = "aaaa日aaaTaaaaaaaaaaaa";
    assert_eq!(period_label(broken), broken);

    assert_eq!(period_label("2026-09-01"), "09-01");
    assert_eq!(period_label("2026-09"), "2026-09");
}
