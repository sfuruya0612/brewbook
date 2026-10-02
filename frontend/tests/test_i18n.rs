//! 翻訳 (FR-16) の単体テスト。
//!
//! - キーの列挙と 2 つの言語の表の対応 (全件)
//! - 日本語の 8 つの訳語
//! - 言語の決定 (端末またはブラウザの言語が日本語なら日本語、それ以外は英語)
//! - `src/ui/` と `src/screens/` の `rsx!` に表示する文字列の直書きが無いこと
//!   (移行前の Flutter 版の `test/l10n_check_test.dart` の検査を写したもの)

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use brew_book_frontend::i18n::{
    resolve_language, set_language, t, t_args, text, Key, Language, KEY_COUNT,
};

#[test]
fn the_japanese_table_has_the_eight_words() {
    for (key, value) in [
        (Key::Producer, "生産者"),
        (Key::Origin, "生産国"),
        (Key::Region, "地域"),
        (Key::Process, "精製方法"),
        (Key::Variety, "品種"),
        (Key::Roast, "焙煎度"),
        (Key::RoastDate, "焙煎日"),
        (Key::FlavorNotes, "フレーバーノート"),
    ] {
        assert_eq!(text(Language::Japanese, key), value, "{key:?}");
    }
}

#[test]
fn the_key_enum_and_both_tables_cover_all_keys() {
    assert_eq!(KEY_COUNT, 227);
    assert_eq!(Key::ALL.len(), KEY_COUNT);
    let unique: HashSet<Key> = Key::ALL.into_iter().collect();
    assert_eq!(unique.len(), KEY_COUNT);

    for (index, key) in Key::ALL.into_iter().enumerate() {
        assert_eq!(key.index(), index, "{key:?}");
        // 表から引けること (panic しないこと)。
        let _ = text(Language::Japanese, key);
        let _ = text(Language::English, key);
    }
    // 日本語の表が空なのは統計の期間の前置きだけ (英語は "from ")。
    let empty: Vec<Key> = Key::ALL
        .into_iter()
        .filter(|key| text(Language::Japanese, *key).is_empty())
        .collect();
    assert_eq!(empty, vec![Key::StatsRangePrefix]);
    assert!(!text(Language::English, Key::StatsRangePrefix).is_empty());
}

#[test]
fn the_language_is_japanese_only_for_japanese() {
    for tag in ["ja", "ja-JP", "ja_JP", "JA", "ja-jp"] {
        assert_eq!(resolve_language(Some(tag)), Language::Japanese, "{tag}");
    }
    for tag in ["en", "en-US", "fr", "zh-Hans", "", "japanese"] {
        assert_eq!(resolve_language(Some(tag)), Language::English, "{tag}");
    }
    assert_eq!(resolve_language(None), Language::English);
}

#[test]
fn the_current_language_switches_the_text() {
    set_language(Language::English);
    assert_eq!(t(Key::SaveButton), text(Language::English, Key::SaveButton));
    set_language(Language::Japanese);
    assert_eq!(t(Key::SaveButton), "保存");
    // 他のテストに影響を残さない。
    set_language(Language::English);
}

#[test]
fn arguments_replace_the_placeholders() {
    set_language(Language::Japanese);
    assert_eq!(
        t_args(
            Key::BrewRowSubtitle,
            &[("date", "2026/10/2"), ("shop", "店")]
        ),
        "2026/10/2 / 店"
    );
    set_language(Language::English);
    // 表に無い名前を渡しても文言は変わらない。
    assert_eq!(
        t_args(Key::SaveButton, &[("unknown", "x")]),
        text(Language::English, Key::SaveButton)
    );
}

#[test]
fn the_language_has_a_code() {
    assert_eq!(Language::Japanese.code(), "ja");
    assert_eq!(Language::English.code(), "en");
}

/// 直書きの検査で見る位置。
#[derive(Clone, PartialEq, Eq, Debug)]
enum Position {
    /// 要素の本文。
    Body,
    /// 表示に使う属性の値 (placeholder、title、aria-label、alt)。
    Attribute(String),
    /// デザインの部品の prop の値 (label、title)。
    Prop(String),
}

/// 表示する文字列の直書きの違反。
#[derive(Clone, PartialEq, Eq, Debug)]
struct Violation {
    /// 行番号 (1 始まり)。
    line: usize,
    /// 直書きされた文字列 (差し込みを除いた本文)。
    literal: String,
    /// 見つけた位置。
    position: Position,
}

/// 表示に使う属性 (この値に文字列リテラルを書かない)。
const DISPLAY_ATTRIBUTES: [&str; 4] = ["placeholder", "title", "aria-label", "alt"];

/// デザインの部品 10 種 (`docs/design/README.md`)。
const DESIGN_COMPONENTS: [&str; 10] = [
    "AppBar",
    "Button",
    "Field",
    "Chip",
    "Rating",
    "ListRow",
    "Ledger",
    "ReferenceTile",
    "Feedback",
    "Charts",
];

/// デザインの部品の prop で表示に使う名前。
const DISPLAY_PROPS: [&str; 2] = ["label", "title"];

/// rsx の中のトークン。
#[derive(Clone, PartialEq, Eq, Debug)]
enum Token {
    /// 識別子 (要素の名前、属性の名前、`::` を含む経路)。
    Word(String),
    /// 文字列リテラル (引用符を除いた中身)。
    Text(String),
    /// 区切り。
    Punct(char),
}

/// コメントと空白を除いたトークンと行番号。
fn tokenize(source: &str) -> Vec<(Token, usize)> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    let mut line = 1;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'\n' {
            line += 1;
            index += 1;
            continue;
        }
        if byte.is_ascii_whitespace() {
            index += 1;
            continue;
        }
        // コメント (行と、入れ子を許すブロック)。
        if byte == b'/' && index + 1 < bytes.len() && bytes[index + 1] == b'/' {
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if byte == b'/' && index + 1 < bytes.len() && bytes[index + 1] == b'*' {
            index += 2;
            let mut depth = 1;
            while index < bytes.len() && depth > 0 {
                if bytes[index] == b'\n' {
                    line += 1;
                }
                if bytes[index] == b'/' && index + 1 < bytes.len() && bytes[index + 1] == b'*' {
                    depth += 1;
                    index += 2;
                } else if bytes[index] == b'*'
                    && index + 1 < bytes.len()
                    && bytes[index + 1] == b'/'
                {
                    depth -= 1;
                    index += 2;
                } else {
                    index += 1;
                }
            }
            continue;
        }
        if byte == b'"' {
            let (text, next) = read_string(source, index);
            line += source[index..next].matches('\n').count();
            tokens.push((Token::Text(text), line));
            index = next;
            continue;
        }
        if byte.is_ascii_alphabetic() || byte == b'_' {
            let start = index;
            while index < bytes.len()
                && (bytes[index].is_ascii_alphanumeric()
                    || bytes[index] == b'_'
                    || bytes[index] == b'-')
            {
                index += 1;
            }
            tokens.push((Token::Word(source[start..index].to_string()), line));
            continue;
        }
        tokens.push((Token::Punct(char::from(byte)), line));
        index += 1;
    }
    tokens
}

/// 引用符で囲まれた文字列を読む (エスケープは次の 1 文字をそのまま入れる)。
fn read_string(source: &str, start: usize) -> (String, usize) {
    let bytes = source.as_bytes();
    let mut value = String::new();
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' if index + 1 < bytes.len() => {
                value.push(char::from(bytes[index + 1]));
                index += 2;
            }
            b'"' => return (value, index + 1),
            _ => {
                let character = source[index..]
                    .chars()
                    .next()
                    .expect("a character must follow");
                value.push(character);
                index += character.len_utf8();
            }
        }
    }
    (value, index)
}

/// ソースの中の `rsx!` のブロックを (開始、終了) の位置で返す。
fn rsx_blocks(tokens: &[(Token, usize)]) -> Vec<(usize, usize)> {
    let mut blocks = Vec::new();
    let mut index = 0;
    while index + 2 < tokens.len() {
        let is_rsx = matches!(&tokens[index].0, Token::Word(word) if word == "rsx")
            && tokens[index + 1].0 == Token::Punct('!')
            && tokens[index + 2].0 == Token::Punct('{');
        if !is_rsx {
            index += 1;
            continue;
        }
        let start = index + 3;
        let mut depth = 1;
        let mut end = start;
        while end < tokens.len() && depth > 0 {
            match tokens[end].0 {
                Token::Punct('{') => depth += 1,
                Token::Punct('}') => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                break;
            }
            end += 1;
        }
        blocks.push((start, end.min(tokens.len())));
        index = end + 1;
    }
    blocks
}

/// ソースの `rsx!` にある表示する文字列の直書きを見つける。
fn find_violations(source: &str) -> Vec<Violation> {
    let tokens = tokenize(source);
    let mut violations = Vec::new();
    for (start, end) in rsx_blocks(&tokens) {
        violations.extend(violations_in_block(&tokens[start..end]));
    }
    violations
}

/// 1 つの `rsx!` のブロックの違反を、現れる順に返す。
///
/// 文脈のスタックで、文字列が要素の本文の直下にあるかどうかを見る。`{` の直前が名前なら
/// 要素の本文、それ以外 (式、クロージャ、マクロの呼び出し) は本文ではない。
fn violations_in_block(tokens: &[(Token, usize)]) -> Vec<Violation> {
    let mut violations = Vec::new();
    let mut contexts: Vec<Option<String>> = Vec::new();
    let mut previous: Vec<&(Token, usize)> = Vec::new();
    for (index, (token, line)) in tokens.iter().enumerate() {
        match token {
            Token::Punct('{') => {
                let name = match previous.last() {
                    Some((Token::Word(word), _)) => Some(word.clone()),
                    _ => None,
                };
                contexts.push(name);
            }
            Token::Punct('}') => {
                contexts.pop();
            }
            Token::Punct('(') | Token::Punct('[') => contexts.push(None),
            Token::Punct(')') | Token::Punct(']') => {
                contexts.pop();
            }
            Token::Text(literal) => {
                let next = tokens.get(index + 1).map(|(token, _)| token);
                let attribute_name = if matches!(previous.last(), Some((Token::Punct(':'), _))) {
                    previous
                        .iter()
                        .rev()
                        .nth(1)
                        .and_then(|(token, _)| match token {
                            Token::Word(word) | Token::Text(word) => Some(word.clone()),
                            _ => None,
                        })
                } else {
                    None
                };
                let position = if let Some(name) = &attribute_name {
                    let in_design_component = contexts
                        .last()
                        .and_then(|context| context.as_deref())
                        .is_some_and(|context| DESIGN_COMPONENTS.contains(&context));
                    if in_design_component && DISPLAY_PROPS.contains(&name.as_str()) {
                        Some(Position::Prop(name.clone()))
                    } else if DISPLAY_ATTRIBUTES.contains(&name.as_str()) {
                        Some(Position::Attribute(name.clone()))
                    } else {
                        None
                    }
                } else if matches!(
                    previous.last(),
                    Some((
                        Token::Punct('{') | Token::Punct('}') | Token::Punct(',') | Token::Text(_),
                        _
                    ))
                ) && contexts.last().is_some_and(|context| context.is_some())
                    && !matches!(next, Some(Token::Punct(':')))
                {
                    // 要素の本文の直下の文字列。本文は子要素 (`}` の後) や別の本文の後にも続く。
                    // 属性の名前を引用符で書いた場合は次が `:` になる。
                    Some(Position::Body)
                } else {
                    None
                };
                if let Some(position) = position {
                    if let Some(text) = displayed_text(literal) {
                        violations.push(Violation {
                            line: *line,
                            literal: text,
                            position,
                        });
                    }
                }
            }
            Token::Word(_) | Token::Punct(_) => {}
        }
        previous.push(&tokens[index]);
        if previous.len() > 2 {
            previous.remove(0);
        }
    }
    violations
}

/// 表示する文字列なら中身を返す。`{name}` の差し込みだけのリテラルと URL は None。
fn displayed_text(literal: &str) -> Option<String> {
    let text = strip_interpolations(literal);
    let text = text.trim();
    if text.is_empty()
        || text.starts_with("http://")
        || text.starts_with("https://")
        || text.starts_with('/')
    {
        return None;
    }
    Some(text.to_string())
}

/// `{...}` の差し込みを取り除く (入れ子の括弧も数える)。
fn strip_interpolations(literal: &str) -> String {
    let mut result = String::new();
    let mut depth = 0_u32;
    for character in literal.chars() {
        match character {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ if depth == 0 => result.push(character),
            _ => {}
        }
    }
    result
}

/// `src/ui/` と `src/screens/` の `.rs` のファイル。
fn ui_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    for directory in ["ui", "screens"] {
        collect_rs_files(&root.join(directory), &mut files);
    }
    files.sort();
    files
}

/// ディレクトリの下の `.rs` のファイルを集める。
fn collect_rs_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

#[test]
fn the_ui_code_does_not_hard_code_displayed_text() {
    let files = ui_files();
    assert!(
        !files.is_empty(),
        "the UI code of src/ui and src/screens must be scanned"
    );

    let mut violations = Vec::new();
    for file in &files {
        let source = fs::read_to_string(file).expect("the source must be readable");
        for violation in find_violations(&source) {
            violations.push(format!(
                "{}:{}: {:?} ({:?})",
                file.display(),
                violation.line,
                violation.literal,
                violation.position
            ));
        }
    }
    assert!(
        violations.is_empty(),
        "the displayed text must come from the translations (FR-16):\n{}",
        violations.join("\n")
    );
}

/// 検査の対象になる位置と、対象外の位置を並べた例。
const SAMPLE: &str = r##"
fn sample() -> Element {
    rsx! {
        div { class: "p-4", id: "screen",
            h1 { "保存" }
            input { placeholder: "名前", title: "題", "aria-label": "名前", alt: "写真" }
            Button { label: "登録する", title: "登録" }
            Field { label: "名前" }
            img { src: "/photo.png", alt: "{t(Key::PhotoLabel)}" }
            a { href: "/settings", "{t(Key::SettingsTitle)}" }
            div { {t(Key::Loading)} }
            div { onclick: move |_| { tracing::info!("saved"); }, "本文" }
            h1 { "見出し" span { "子" } "直書き1" }
            div { "直書き2" "直書き3" }
            div { {t(Key::Loading)} "直書き4" }
        }
    }
}
"##;

#[test]
fn the_check_finds_every_displayed_position() {
    let violations = find_violations(SAMPLE);
    let found: Vec<(String, Position)> = violations
        .into_iter()
        .map(|violation| (violation.literal, violation.position))
        .collect();
    assert_eq!(
        found,
        vec![
            ("保存".to_string(), Position::Body),
            (
                "名前".to_string(),
                Position::Attribute("placeholder".to_string())
            ),
            ("題".to_string(), Position::Attribute("title".to_string())),
            (
                "名前".to_string(),
                Position::Attribute("aria-label".to_string())
            ),
            ("写真".to_string(), Position::Attribute("alt".to_string())),
            ("登録する".to_string(), Position::Prop("label".to_string())),
            ("登録".to_string(), Position::Prop("title".to_string())),
            ("名前".to_string(), Position::Prop("label".to_string())),
            ("本文".to_string(), Position::Body),
            // 子要素の後、別の本文の後、式ブロックの後の本文も検出する。
            ("見出し".to_string(), Position::Body),
            ("子".to_string(), Position::Body),
            ("直書き1".to_string(), Position::Body),
            ("直書き2".to_string(), Position::Body),
            ("直書き3".to_string(), Position::Body),
            ("直書き4".to_string(), Position::Body),
        ]
    );
}

#[test]
fn the_check_ignores_the_other_attributes_the_urls_and_the_interpolations() {
    let clean = r##"
fn sample() -> Element {
    rsx! {
        div { class: "p-4", id: "screen", role: "main",
            h1 { "{t(Key::HomeTitle)}" }
            input { type: "text", name: "name", autocomplete: "off", value: "{value}" }
            Button { label: "{t(Key::SaveButton)}", onclick: move |_| { tracing::info!("saved"); } }
            a { href: "/settings", "{t(Key::SettingsTitle)}" }
            img { src: "https://example.com/photo.png", alt: "{t(Key::PhotoLabel)}" }
            div { {t(Key::Loading)} }
        }
    }
}
"##;
    assert_eq!(find_violations(clean), Vec::new());
}
