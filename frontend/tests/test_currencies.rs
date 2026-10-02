//! `records::currencies` の単体テスト (0041)。
//!
//! 通貨のマスタと、プルダウンの選択肢の組み立て (0041 のレビューの指摘)。

use brew_book_frontend::i18n::Language;
use brew_book_frontend::records::{currency_name, currency_option_label, currency_options};

#[test]
fn the_options_are_sorted_and_include_the_current_value() {
    let options = currency_options(None);
    assert!(options.contains(&"JPY".to_string()));
    assert!(options.contains(&"USD".to_string()));
    let mut sorted = options.clone();
    sorted.sort();
    assert_eq!(options, sorted, "the options must be sorted");

    // マスタにあるコードは重複しない。
    let jpy = currency_options(Some("JPY"));
    assert_eq!(jpy.iter().filter(|code| code.as_str() == "JPY").count(), 1);

    // マスタに無いコード (旧い記録) は足して選べるようにする (編集の互換)。
    let unknown = currency_options(Some("XYZ"));
    assert!(unknown.contains(&"XYZ".to_string()));
    let mut sorted = unknown.clone();
    sorted.sort();
    assert_eq!(unknown, sorted, "the options must stay sorted");
}

#[test]
fn the_names_and_labels_come_from_the_master() {
    assert_eq!(currency_name(Language::Japanese, "JPY"), "日本円");
    // マスタに無いコードはコードのままにする。
    assert_eq!(currency_name(Language::Japanese, "XYZ"), "XYZ");
    assert_eq!(
        currency_option_label(Language::Japanese, "JPY"),
        "JPY 日本円"
    );
    assert_eq!(currency_option_label(Language::Japanese, "XYZ"), "XYZ");
}
