//! `ui::rating` の PBT。評価の丸の点灯の数 (ADR-0013)。

#![cfg(not(target_arch = "wasm32"))]

use brew_book_frontend::ui::lit_dots;
use proptest::prelude::*;

proptest! {
    /// 点灯の数は 0 から 5 に収まり、値が増えると減らない。未評価 (None) は 0。
    #[test]
    fn the_lit_dots_clamp_and_are_monotonic(low in any::<u8>(), high in any::<u8>()) {
        prop_assume!(low <= high);
        prop_assert_eq!(lit_dots(None), 0);
        prop_assert!(lit_dots(Some(low)) <= lit_dots(Some(high)));
        prop_assert!(lit_dots(Some(high)) <= 5);
    }

    /// 1 から 5 の評価は、その数だけ点灯する。
    #[test]
    fn a_valid_rating_lights_the_same_number_of_dots(value in 1..=5u8) {
        prop_assert_eq!(lit_dots(Some(value)), value);
    }
}
