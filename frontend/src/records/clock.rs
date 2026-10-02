//! 端末の時計とタイムゾーンの依存 (FR-9、FR-11、FR-18)。
//!
//! 統計の画面は、端末のタイムゾーンでの当月の期間を組み立て、端末のローカル時刻から
//! UTC を引いた分数を API に渡す。テストが端末の時計とタイムゾーンを差し替えられるよう、
//! この型で画面へ配る (ADR-0007)。

use super::values::LocalDateTime;

/// 実行環境の時計とタイムゾーン。
pub trait Clock {
    /// 端末のタイムゾーンでの現在の日時。
    fn now(&self) -> LocalDateTime;

    /// 端末のローカル時刻から UTC を引いた分数 (FR-18)。日本標準時は +540。
    fn utc_offset_minutes(&self) -> i32;
}

/// ブラウザの時計とタイムゾーン。
#[cfg(target_arch = "wasm32")]
pub struct DeviceClock;

#[cfg(target_arch = "wasm32")]
impl Clock for DeviceClock {
    fn now(&self) -> LocalDateTime {
        let date = js_sys::Date::new_0();
        LocalDateTime::new(
            date.get_full_year() as i32,
            date.get_month() as u32 + 1,
            date.get_date() as u32,
            date.get_hours() as u32,
            date.get_minutes() as u32,
        )
    }

    fn utc_offset_minutes(&self) -> i32 {
        // JS の getTimezoneOffset は UTC からの遅れ (分)。日本標準時は -540 なので符号を反転する。
        let date = js_sys::Date::new_0();
        -(date.get_timezone_offset() as i32)
    }
}
