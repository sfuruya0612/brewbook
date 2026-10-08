//! `records::maps` の PBT (FR-22、ADR-0019)。
//!
//! 地図の URL の組み立てが、住所のどんな文字でも壊れないこと (住所がパーセントエンコードされ、
//! URL の区切りと衝突しないこと) を確認する。

#![cfg(not(target_arch = "wasm32"))]

use brew_book_frontend::i18n::Language;
use brew_book_frontend::records::map_embed_url;
use proptest::prelude::*;

/// URL の住所の部分を取り出す前置きと後置き。
const PREFIX: &str = "https://www.google.com/maps/embed/v1/place?key=test-key&q=";
const SUFFIX: &str = "&language=ja";

proptest! {
    /// 任意の住所の URL は、住所をエンコードした安全な文字だけを含む。
    #[test]
    fn the_map_url_encodes_any_address(address in any::<String>()) {
        let url = map_embed_url("test-key", &address, Language::Japanese);
        prop_assert!(url.starts_with(PREFIX), "{url}");
        prop_assert!(url.ends_with(SUFFIX), "{url}");
        let query = &url[PREFIX.len()..url.len() - SUFFIX.len()];
        // エンコードの結果は unreserved (RFC 3986) と `%XX` だけになる。
        prop_assert!(
            query
                .chars()
                .all(|character| character.is_ascii_alphanumeric()
                    || "-._~%".contains(character)),
            "the query must be percent-encoded: {query}"
        );
    }

    /// 言語は `ja` と `en` のどちらでも URL の末尾にそのまま入る。
    #[test]
    fn the_map_url_carries_the_language(address in any::<String>(), japanese in any::<bool>()) {
        let language = if japanese { Language::Japanese } else { Language::English };
        let url = map_embed_url("test-key", &address, language);
        prop_assert!(url.ends_with(&format!("&language={}", language.code())), "{url}");
    }
}
