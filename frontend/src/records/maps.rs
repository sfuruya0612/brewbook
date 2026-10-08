//! 店の地図 (FR-22、ADR-0019) の URL の組み立て。
//!
//! 地図は Maps Embed API の iframe で表示する。キーは `GET /api/maps/config` で配られ、
//! 住所は Google のジオコーディングに任せる (座標は保存しない。ADR-0019)。
//! ここは Dioxus に依存しない (ADR-0013)。

use crate::i18n::Language;

use super::api::encode_query;

/// 地図の埋め込みの URL を組み立てる (FR-22)。
///
/// `q` は場所の名前、住所、プラスコード、プレイス ID を受け付けるため、住所をそのまま渡す。
/// 住所と言語はパーセントエンコードする。
pub fn map_embed_url(key: &str, address: &str, language: Language) -> String {
    format!(
        "https://www.google.com/maps/embed/v1/place?key={}&q={}&language={}",
        encode_query(key),
        encode_query(address),
        language.code()
    )
}
