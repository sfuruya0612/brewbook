//! デザインシステムの部品 (0039)。
//!
//! `docs/design/` の視覚言語を Tailwind CSS v4 の `@theme` と `src/ui/design.css` に写し、
//! 部品 10 種 (AppBar、Button、Field、Chip、Rating、ListRow、Ledger、ReferenceTile、Feedback、
//! Charts) と 2 段組のレイアウト ([`WideLayout`]) を `rsx!` で組む。
//!
//! クラス名は `docs/design/components/bundle.css` と同じにして、原本との突き合わせを機械的に
//! できるようにする (0039 の設計判断)。値は `docs/design/tokens.css` の変数だけを参照する。
//! `rsx!` に表示する文字列を直書きしないことは、`frontend/tests/test_i18n.rs` が検査する (FR-16)。

mod app_bar;
mod button;
mod charts;
mod chip;
mod feedback;
mod field;
mod icon;
mod ledger;
mod list_row;
mod mark;
mod rating;
mod reference_tile;
mod wide_layout;

use dioxus::prelude::{Key, KeyboardEvent};

/// Enter か Space で押されたか (button の動きに合わせる)。
///
/// `role="button"` を付けた要素はキーボードでも押せる必要がある (WAI-ARIA の button の
/// 要件。0039 のレビューの指摘)。`onkeydown` から使う。
pub(crate) fn is_activation_key(event: &KeyboardEvent) -> bool {
    match event.key() {
        Key::Enter => true,
        Key::Character(ref value) => value == " ",
        _ => false,
    }
}

pub use app_bar::AppBar;
pub use button::{Button, ButtonSize, ButtonVariant, Fab, IconButton};
pub use charts::{
    ChartBars, ChartFrame, ChartLine, ChartScatter, ChartScatterFrame, ChartSection, ChartSeries,
    StatTile, StatTiles,
};
pub use chip::{Chip, ChipVariant, TagChip};
pub use feedback::{Banner, ConfirmDialog, Snackbar};
pub use field::{Field, TextField, TextFieldKind};
pub use icon::Icon;
pub use ledger::{Ledger, LedgerRow};
pub use list_row::{ListRow, ListThumb, RowValue};
pub use mark::BrewbookMark;
pub use rating::{lit_dots, Rating, RatingInput, RatingSize};
pub use reference_tile::{PickerTile, ReferenceChain, ReferenceTile};
pub use wide_layout::{NavigationRail, RailItem, WideLayout, WidePage};
