//! グラフの座標の計算 (FR-18)。
//!
//! 値から SVG の座標への変換と目盛りの刻みを純粋な関数として切り出す。画面はこの結果を
//! そのまま `rsx!` の属性に置き、描画の規則を native の単体テストと PBT で守れるようにする
//! (ADR-0013)。
//!
//! 枠は `ui::ChartFrame` と同じ 340 x 120 の viewBox にする。目盛りの横罫は最大 (8) と
//! 半分 (50)、軸は値の 0 と評価の 1 (92) の位置に引く (docs/design/components/Charts)。

/// グラフの枠の幅 (viewBox)。
pub const FRAME_WIDTH: f64 = 340.0;

/// グラフの枠の高さ (viewBox)。
pub const FRAME_HEIGHT: f64 = 120.0;

/// 棒グラフの縦軸の左端 (目盛りの数字の幅)。
pub const PLOT_LEFT: f64 = 28.0;

/// 散布図の縦軸の左端 (デザインのプレビューと同じ)。
pub const SCATTER_LEFT: f64 = 22.0;

/// 枠の右端。
pub const PLOT_RIGHT: f64 = 340.0;

/// 目盛りの上の線の位置 (最大)。
pub const PLOT_TOP: f64 = 8.0;

/// 軸の位置 (値の 0 と評価の 1)。
pub const PLOT_BOTTOM: f64 = 92.0;

/// 評価の下限 (FR-18)。
pub const RATING_MIN: f64 = 1.0;

/// 評価の上限 (FR-18)。
pub const RATING_MAX: f64 = 5.0;

/// 棒の幅の上限 (デザインのプレビューと同じ)。
pub const MAX_BAR_WIDTH: f64 = 19.0;

/// 折れ線の最初の点の位置。
pub const LINE_LEFT: f64 = 44.0;

/// 折れ線の最後の点の位置。
pub const LINE_RIGHT: f64 = 320.0;

/// 値の軸の上限。値が無いか 0 以下なら 1 にし、そうでなければ最大の 1.1 倍にする
/// (Flutter の fl_chart の組み立てと同じ)。
pub fn axis_max(max_value: f64) -> f64 {
    if max_value > 0.0 {
        max_value * 1.1
    } else {
        1.0
    }
}

/// 目盛りの値 (0、半分、最大)。下から上の順に返す。
pub fn axis_ticks(max_value: f64) -> [f64; 3] {
    let max = axis_max(max_value);
    [0.0, max / 2.0, max]
}

/// 目盛りの縦の位置。`fraction` は軸の下 (0) から上 (1)。
pub fn tick_y(fraction: f64) -> f64 {
    PLOT_BOTTOM - fraction.clamp(0.0, 1.0) * (PLOT_BOTTOM - PLOT_TOP)
}

/// 棒グラフの棒の左端と幅。区間の数だけ返す。
///
/// 棒の間に 3 の隙間を置き、幅は [`MAX_BAR_WIDTH`] を上限にする (デザインのプレビューと同じ)。
pub fn bar_slots(count: usize) -> Vec<(f64, f64)> {
    if count == 0 {
        return Vec::new();
    }
    let slot = (PLOT_RIGHT - PLOT_LEFT - 4.0) / count as f64;
    let width = (slot - 3.0).clamp(1.0, MAX_BAR_WIDTH);
    (0..count)
        .map(|index| (PLOT_LEFT + 1.5 + index as f64 * slot, width))
        .collect()
}

/// 棒の上端と高さ。`max_value` は区間の値の最大。
pub fn bar_rect(value: f64, max_value: f64) -> (f64, f64) {
    let ratio = (value / axis_max(max_value)).clamp(0.0, 1.0);
    let height = ratio * (PLOT_BOTTOM - PLOT_TOP);
    (PLOT_BOTTOM - height, height)
}

/// 目盛りの値の表示。整数のときはそのまま、そうでなければ小数第 1 位まで出す
/// (Flutter の `_formatAxis` と同じ)。
pub fn format_axis(value: f64) -> String {
    if value == value.round() {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

/// 区間のキーが日別か (日別のキーは `YYYY-MM-DD` の 10 文字)。
///
/// ASCII だけを日別とみなす (壊れた応答で文字の境界を切らないため。0042 のレビューの指摘)。
pub fn is_daily_key(key: &str) -> bool {
    key.len() == 10 && key.is_ascii()
}

/// 区間のキーまたは抽出日時の表示。日付の部分の月日 (`MM-DD`) だけにする。
///
/// 月別のキー (`YYYY-MM`) はそのまま返す。応答が壊れていて日付の形をしていないときも、
/// panic せずに元の文字列を返す (0042 のレビューの指摘)。
pub fn period_label(key: &str) -> String {
    if is_daily_key(key) || is_timestamp(key) {
        key.get(5..10).unwrap_or(key).to_string()
    } else {
        key.to_string()
    }
}

/// ISO 8601 の UTC の日時 (`YYYY-MM-DDTHH:MM:SS.mmmZ`、24 文字) か。
fn is_timestamp(key: &str) -> bool {
    key.len() == 24 && key.is_ascii() && key.as_bytes().get(10) == Some(&b'T')
}

/// 下の軸のラベルを出す位置 (先頭、中央、末尾。3 つ以下なら全部)。
pub fn label_indices(count: usize) -> Vec<usize> {
    if count <= 3 {
        (0..count).collect()
    } else {
        vec![0, count / 2, count - 1]
    }
}

/// 散布図の横軸の範囲。最小と最大が同じときは前後 1 に広げる (Flutter と同じ)。
pub fn scatter_x_range(min: f64, max: f64) -> (f64, f64) {
    if min == max {
        (min - 1.0, max + 1.0)
    } else {
        (min, max)
    }
}

/// 散布図の横軸の位置。
pub fn scatter_x(value: f64, min: f64, max: f64) -> f64 {
    if max <= min {
        return SCATTER_LEFT;
    }
    SCATTER_LEFT + (value - min) / (max - min) * (PLOT_RIGHT - SCATTER_LEFT)
}

/// 評価の縦の位置 (1 が下、5 が上)。
pub fn rating_y(rating: f64) -> f64 {
    tick_y((rating - RATING_MIN) / (RATING_MAX - RATING_MIN))
}

/// 折れ線の点の横の位置。1 点のときは中央に置く。
pub fn line_x(index: usize, count: usize) -> f64 {
    if count <= 1 {
        return (LINE_LEFT + LINE_RIGHT) / 2.0;
    }
    LINE_LEFT + index as f64 / (count - 1) as f64 * (LINE_RIGHT - LINE_LEFT)
}
