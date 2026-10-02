//! `records::chart` の PBT (0042)。グラフの座標の計算の性質 (ADR-0013)。

#![cfg(not(target_arch = "wasm32"))]

use brew_book_frontend::records::chart::{
    axis_max, axis_ticks, bar_rect, bar_slots, label_indices, line_x, period_label, rating_y,
    scatter_x, scatter_x_range, tick_y, LINE_LEFT, LINE_RIGHT, PLOT_BOTTOM, PLOT_LEFT, PLOT_RIGHT,
    PLOT_TOP, RATING_MAX, RATING_MIN, SCATTER_LEFT,
};
use brew_book_frontend::records::values::{format_day, LocalDate};
use proptest::prelude::*;

/// 軸の基準になる正の値。
fn positive_value() -> impl Strategy<Value = f64> {
    (1u32..=10_000).prop_map(f64::from)
}

/// 端末のローカル日時 (実在する日付)。
fn valid_date() -> impl Strategy<Value = LocalDate> {
    (0..=9999i32, 1..=12u32, 1..=28u32)
        .prop_map(|(year, month, day)| LocalDate::new(year, month, day))
}

proptest! {
    /// 軸の上限は値の最大以上で、1.1 倍を超えない。
    #[test]
    fn the_axis_max_covers_the_value(max_value in positive_value()) {
        let max = axis_max(max_value);
        prop_assert!(max >= max_value, "{} < {}", max, max_value);
        prop_assert!(max <= max_value * 1.1 + f64::EPSILON);
    }

    /// 目盛りの値は 0 から軸の上限までの昇順。
    #[test]
    fn the_axis_ticks_are_ordered(max_value in positive_value()) {
        let [zero, half, max] = axis_ticks(max_value);
        prop_assert_eq!(zero, 0.0);
        prop_assert!(half > zero);
        prop_assert!(max > half);
        prop_assert!(max >= max_value);
    }

    /// 目盛りの位置は軸 (0) から上 (1) の間にあり、単調に上がる。
    #[test]
    fn the_tick_positions_stay_in_the_plot(lower in 0u32..=100, higher in 0u32..=100) {
        let (lower, higher) = if lower <= higher { (lower, higher) } else { (higher, lower) };
        let y_lower = tick_y(f64::from(lower) / 100.0);
        let y_higher = tick_y(f64::from(higher) / 100.0);
        prop_assert!((PLOT_TOP..=PLOT_BOTTOM).contains(&y_lower));
        prop_assert!((PLOT_TOP..=PLOT_BOTTOM).contains(&y_higher));
        prop_assert!(y_lower >= y_higher, "{} < {}", y_lower, y_higher);
    }

    /// 棒は区間の数だけ並び、枠の中に収まり、重ならない。
    #[test]
    fn the_bars_stay_within_the_plot(count in 1usize..=64) {
        let slots = bar_slots(count);
        prop_assert_eq!(slots.len(), count);
        let mut previous_end = PLOT_LEFT;
        for (x, width) in slots {
            prop_assert!(width >= 1.0);
            prop_assert!(x >= PLOT_LEFT, "{} < {}", x, PLOT_LEFT);
            prop_assert!(x + width <= PLOT_RIGHT, "{} > {}", x + width, PLOT_RIGHT);
            prop_assert!(x >= previous_end, "{} < {}", x, previous_end);
            previous_end = x + width;
        }
    }

    /// 棒の上端と高さは軸に接し、値が大きいほど上になる。
    #[test]
    fn the_bar_height_follows_the_value(
        max_value in positive_value(),
        first in 0u32..=10_000,
        second in 0u32..=10_000,
    ) {
        let first = f64::from(first).min(max_value);
        let second = f64::from(second).min(max_value);
        let (y_first, height_first) = bar_rect(first, max_value);
        let (y_second, height_second) = bar_rect(second, max_value);
        prop_assert!((y_first + height_first - PLOT_BOTTOM).abs() < 1e-9);
        prop_assert!((y_second + height_second - PLOT_BOTTOM).abs() < 1e-9);
        prop_assert!((PLOT_TOP..=PLOT_BOTTOM).contains(&y_first));
        prop_assert!((PLOT_TOP..=PLOT_BOTTOM).contains(&y_second));
        if first <= second {
            prop_assert!(y_first >= y_second, "{} < {}", y_first, y_second);
        } else {
            prop_assert!(y_first <= y_second, "{} > {}", y_first, y_second);
        }
    }

    /// 散布図の点は枠の中に収まり、値の昇順に右へ動く。
    #[test]
    fn the_scatter_points_stay_within_the_plot(
        min in -1_000i32..=1_000,
        span in 1u32..=1_000,
        first in 0u32..=1_000,
        second in 0u32..=1_000,
    ) {
        let min = f64::from(min);
        let max = min + f64::from(span);
        let first = min + f64::from(first) % (f64::from(span) + 1.0);
        let second = min + f64::from(second) % (f64::from(span) + 1.0);
        let x_first = scatter_x(first, min, max);
        let x_second = scatter_x(second, min, max);
        prop_assert!((SCATTER_LEFT..=PLOT_RIGHT).contains(&x_first));
        prop_assert!((SCATTER_LEFT..=PLOT_RIGHT).contains(&x_second));
        if first <= second {
            prop_assert!(x_first <= x_second);
        } else {
            prop_assert!(x_first >= x_second);
        }
    }

    /// 同じ値だけの散布図は前後 1 に広げる。
    #[test]
    fn the_scatter_range_is_widened_when_the_values_are_equal(value in -1_000i32..=1_000) {
        let value = f64::from(value);
        prop_assert_eq!(scatter_x_range(value, value), (value - 1.0, value + 1.0));
    }

    /// 評価の位置は 1 が下、5 が上で、その間に収まる。
    #[test]
    fn the_rating_position_follows_the_rating(rating in 1u32..=5) {
        prop_assert_eq!(rating_y(RATING_MIN), PLOT_BOTTOM);
        prop_assert_eq!(rating_y(RATING_MAX), PLOT_TOP);
        let y = rating_y(f64::from(rating));
        prop_assert!((PLOT_TOP..=PLOT_BOTTOM).contains(&y));
        if rating < 5 {
            prop_assert!(y > rating_y(f64::from(rating + 1)));
        }
    }

    /// 折れ線の点は両端を固定して等間隔に並ぶ。
    ///
    /// 期待値は実装の式を写さず、隣り合う点の間隔が等しい性質で検査する (0042 のレビューの
    /// 指摘。ADR-0013 の方針)。
    #[test]
    fn the_line_points_are_evenly_spread(count in 2usize..=100) {
        prop_assert_eq!(line_x(0, count), LINE_LEFT);
        prop_assert_eq!(line_x(count - 1, count), LINE_RIGHT);
        let step = line_x(1, count) - line_x(0, count);
        prop_assert!(step > 0.0, "the points must go from left to right");
        for index in 1..count - 1 {
            let gap = line_x(index + 1, count) - line_x(index, count);
            prop_assert!(
                (gap - step).abs() < 1e-9,
                "the gaps must be even: {} != {}",
                gap,
                step
            );
        }
    }

    /// 軸のラベルの位置は昇順で重複せず、先頭と末尾を含み、区間の数に収まる。
    #[test]
    fn the_label_indices_are_ordered_and_inside(count in 1usize..=100) {
        let indices = label_indices(count);
        prop_assert_eq!(indices.first(), Some(&0));
        prop_assert_eq!(indices.last(), Some(&(count - 1)));
        for index in &indices {
            prop_assert!(*index < count);
        }
        for pair in indices.windows(2) {
            prop_assert!(pair[0] < pair[1]);
        }
    }

    /// 日別のキーの表示は月日になる。
    #[test]
    fn the_period_label_is_the_month_and_day(date in valid_date()) {
        let label = period_label(&format_day(date));
        prop_assert_eq!(label, format!("{:02}-{:02}", date.month, date.day));
    }
}
