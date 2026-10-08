//! `records::maps` の単体テスト (FR-22、ADR-0019)。
//!
//! 地図の埋め込みの URL の組み立て (キーと住所のパーセントエンコード、言語) を確認する。

use brew_book_frontend::i18n::Language;
use brew_book_frontend::records::map_embed_url;

#[test]
fn the_map_url_encodes_the_key_and_the_address() {
    assert_eq!(
        map_embed_url("test-key", "長野県北佐久郡軽井沢町", Language::Japanese),
        "https://www.google.com/maps/embed/v1/place?key=test-key&q=%E9%95%B7%E9%87%8E%E7%9C%8C%E5%8C%97%E4%BD%90%E4%B9%85%E9%83%A1%E8%BB%BD%E4%BA%95%E6%B2%A2%E7%94%BA&language=ja"
    );
    // 空白と記号もエンコードする (空白は %20)。
    assert_eq!(
        map_embed_url("key", "1-2-3 Main St, Seattle WA", Language::English),
        "https://www.google.com/maps/embed/v1/place?key=key&q=1-2-3%20Main%20St%2C%20Seattle%20WA&language=en"
    );
    // キーの記号もエンコードする (Google の API キーは通常は英数字と `-` と `_` だけである)。
    assert_eq!(
        map_embed_url("a b&c", "住所", Language::Japanese),
        "https://www.google.com/maps/embed/v1/place?key=a%20b%26c&q=%E4%BD%8F%E6%89%80&language=ja"
    );
}
