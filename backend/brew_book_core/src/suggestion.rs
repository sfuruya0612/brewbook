//! 写真から購入と商品の項目を推測するプロンプトと、モデルの応答の解析 (FR-19、ADR-0016)。
//!
//! プロンプトは応答の JSON の形を固定し、項目を増やさない (出力の tokens を抑えるため)。
//! 解析は、前後の説明文やコードフェンスを許して最初の JSON オブジェクトを取り出し、
//! サーバー側で項目ごとに検証する。形式に合わない項目だけを null にし、他の項目は返す。
//! 解析できないときは全項目を null にする。
//!
//! Workers AI の呼び出しは Worker 側 (`brew_book::records::purchase_suggestions`) が行う。
//! ここは wasm に依存せず、ネイティブのテストで検証する (ADR-0001)。

use serde::{Deserialize, Serialize};

use crate::datetime;
use crate::records;

/// 商品名の上限 (文字数)。
pub const MAX_NAME_CHARS: usize = 200;
/// 商品名以外の文字列の上限 (文字数)。
pub const MAX_TEXT_CHARS: usize = 100;
/// Flavor Notes の上限 (件数)。
pub const MAX_FLAVOR_NOTES: usize = 20;

/// モデルに渡すプロンプト (FR-19)。
///
/// 出力の tokens を抑えるため、応答の JSON の形を固定し、項目を増やさない (ADR-0016)。
const PROMPT: &str = r#"You read a photo of a coffee bag and extract the purchase and product information printed on the package.
Reply with exactly one JSON object and nothing else. Do not explain and do not wrap the JSON in a code fence.
The JSON object has exactly these keys:
{"product": {"name": string|null, "producer": string|null, "origin": string|null, "region": string|null, "process": string|null, "variety": string|null, "flavor_notes": string[]}, "roast": string|null, "roast_date": string|null, "price_amount": integer|null, "weight_grams": integer|null}
Rules:
- Use null for a value that is not printed on the package or that you cannot read.
- Set product to null when the package shows no product information.
- Copy the values as printed on the package and keep each one at most 100 characters long.
- roast_date must be a real date in the format YYYY-MM-DD.
- price_amount and weight_grams must be integers of 0 or more, without currency symbols or units.
- flavor_notes is an array of at most 20 short flavor descriptions. Use an empty array when none are printed.
- Do not add any keys other than the ones above."#;

/// モデルに渡すプロンプトを返す (FR-19)。
pub fn prompt() -> &'static str {
    PROMPT
}

/// 推測した商品の項目 (FR-19)。推測できない項目は None にする。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductSuggestion {
    /// 商品名。上限は 200 文字。
    pub name: Option<String>,
    /// 生産者。上限は 100 文字。
    pub producer: Option<String>,
    /// 生産国。上限は 100 文字。
    pub origin: Option<String>,
    /// 地域。上限は 100 文字。
    pub region: Option<String>,
    /// 精製方法。上限は 100 文字。
    pub process: Option<String>,
    /// 品種。上限は 100 文字。
    pub variety: Option<String>,
    /// Flavor Notes の候補。上限は 20 件 (FR-8)。
    #[serde(default)]
    pub flavor_notes: Vec<String>,
}

impl ProductSuggestion {
    /// 推測できた項目が 1 つも無いか。無いときは応答の `product` を null にする (FR-19)。
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.producer.is_none()
            && self.origin.is_none()
            && self.region.is_none()
            && self.process.is_none()
            && self.variety.is_none()
            && self.flavor_notes.is_empty()
    }
}

/// 推測の応答 (FR-19)。キーは既存の API の列名に揃える。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PurchaseSuggestion {
    /// 商品の項目。1 つも推測できないときは null。
    pub product: Option<ProductSuggestion>,
    /// 焙煎度。
    pub roast: Option<String>,
    /// 焙煎日 (`YYYY-MM-DD`)。
    pub roast_date: Option<String>,
    /// 価格 (整数)。通貨は推測しない (画面の既定値の JPY のまま)。
    pub price_amount: Option<i64>,
    /// 重量 (グラム)。
    pub weight_grams: Option<i64>,
}

/// AI バインディングの応答から、モデルの出力のテキストを取り出す (FR-19)。
///
/// モデルにより応答の形が異なるため、両方を扱う。
///
/// - `response` に文字列で返る形 (古いモデル)
/// - OpenAI 互換の `choices[0].message.content` に文字列で返る形 (新しいモデル)
///
/// `response` がオブジェクトのときは、その JSON をそのまま解析にかける。
/// `response` が空の文字列のときは `choices` に落とす (両方のキーを持つモデルのため)。
/// どちらも無いときは空の文字列を返す (解析の結果は全項目 null になる)。
pub fn output_text(result: &serde_json::Value) -> String {
    if let Some(value) = result.get("response") {
        match value {
            serde_json::Value::String(text) => {
                if !text.trim().is_empty() {
                    return text.clone();
                }
            }
            serde_json::Value::Null => {}
            other => return other.to_string(),
        }
    }
    result
        .get("choices")
        .and_then(|value| value.as_array())
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(|content| content.as_str())
        .map(str::to_owned)
        .unwrap_or_default()
}

/// モデルの応答を解析して推測にする (FR-19)。
///
/// 前後の説明文やコードフェンスを許して最初の JSON オブジェクトを取り出す。
/// 解析できないときと、項目の形式が合わないときは、その項目を None にする。
pub fn parse_response(text: &str) -> PurchaseSuggestion {
    let Some(value) = first_json_object(text) else {
        return PurchaseSuggestion::default();
    };
    parse_object(&value)
}

/// 画像を `data:image/jpeg;base64,...` の URL にする (Workers AI の messages に渡す)。
pub fn image_data_url(bytes: &[u8]) -> String {
    let mut url = String::with_capacity(bytes.len().div_ceil(3) * 4 + DATA_URL_PREFIX.len());
    url.push_str(DATA_URL_PREFIX);
    push_base64(&mut url, bytes);
    url
}

/// `data:` URL の接頭辞。入力は変換済みの JPEG だけとする (FR-10、FR-19)。
const DATA_URL_PREFIX: &str = "data:image/jpeg;base64,";

/// 標準の base64 (RFC 4648 の Section 4) に符号化する。末尾のパディングを付ける。
fn push_base64(out: &mut String, bytes: &[u8]) {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        out.push(char::from(ALPHABET[usize::from(first >> 2)]));
        out.push(char::from(
            ALPHABET[usize::from((first & 0x03) << 4 | second >> 4)],
        ));
        if chunk.len() > 1 {
            out.push(char::from(
                ALPHABET[usize::from((second & 0x0f) << 2 | third >> 6)],
            ));
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(char::from(ALPHABET[usize::from(third & 0x3f)]));
        } else {
            out.push('=');
        }
    }
}

/// 文字列の中から最初の JSON オブジェクトを取り出す。
///
/// 最初の `{` から順に、そこで始まる JSON を読めるか試す。説明文の中の `{` や、
/// 壊れた JSON の後に続くオブジェクトを飛ばせるようにするためである。
fn first_json_object(text: &str) -> Option<serde_json::Value> {
    for (index, byte) in text.bytes().enumerate() {
        if byte != b'{' {
            continue;
        }
        // `{` は ASCII のため、この位置は常に文字の境界になる。
        let mut stream =
            serde_json::Deserializer::from_str(&text[index..]).into_iter::<serde_json::Value>();
        if let Some(Ok(value)) = stream.next() {
            if value.is_object() {
                return Some(value);
            }
        }
    }
    None
}

/// JSON のオブジェクトを推測にする。項目ごとに検証し、形式に合わない項目は None にする。
fn parse_object(value: &serde_json::Value) -> PurchaseSuggestion {
    let product = value
        .get("product")
        .filter(|product| product.is_object())
        .map(|product| ProductSuggestion {
            name: text_field(product, "name", MAX_NAME_CHARS),
            producer: text_field(product, "producer", MAX_TEXT_CHARS),
            origin: text_field(product, "origin", MAX_TEXT_CHARS),
            region: text_field(product, "region", MAX_TEXT_CHARS),
            process: text_field(product, "process", MAX_TEXT_CHARS),
            variety: text_field(product, "variety", MAX_TEXT_CHARS),
            flavor_notes: flavor_notes_field(product),
        })
        // 商品の項目が 1 つも推測できないときは product を null にする (FR-19)。
        .filter(|product| !product.is_empty());
    PurchaseSuggestion {
        product,
        roast: text_field(value, "roast", MAX_TEXT_CHARS),
        roast_date: date_field(value, "roast_date"),
        price_amount: count_field(value, "price_amount"),
        weight_grams: count_field(value, "weight_grams"),
    }
}

/// 文字列の項目を読む。
/// 文字列でない値、前後の空白を除いて空になる値、長すぎる値は None にする。
fn text_field(object: &serde_json::Value, key: &str, max_chars: usize) -> Option<String> {
    let text = object.get(key)?.as_str()?.trim();
    if text.is_empty() || text.chars().count() > max_chars {
        return None;
    }
    Some(text.to_owned())
}

/// 日付の項目を読む。`YYYY-MM-DD` の実在する日付だけを受け付ける (FR-9 と同じ形式)。
fn date_field(object: &serde_json::Value, key: &str) -> Option<String> {
    let text = text_field(object, key, MAX_TEXT_CHARS)?;
    datetime::is_valid_date(&text).then_some(text)
}

/// 0 以上の整数の項目を読む。整数でない値と、保存できない範囲の値は None にする。
/// 小数を持たない数値 (200.0 など) は整数として扱う。
fn count_field(object: &serde_json::Value, key: &str) -> Option<i64> {
    let value = match object.get(key)? {
        serde_json::Value::Number(number) => match number.as_i64() {
            Some(value) => value,
            None => {
                let float = number.as_f64()?;
                if !float.is_finite() || float.fract() != 0.0 {
                    return None;
                }
                // 範囲の検査は下の validate_count が行う (飽和する変換でも安全)。
                float as i64
            }
        },
        _ => return None,
    };
    records::validate_count(value).ok()
}

/// Flavor Notes を読む。文字列の配列だけを受け付け、空の名前と長すぎる名前を除き、
/// 20 件までにする (FR-8、FR-19)。
fn flavor_notes_field(object: &serde_json::Value) -> Vec<String> {
    let Some(items) = object
        .get("flavor_notes")
        .and_then(|value| value.as_array())
    else {
        return Vec::new();
    };
    let mut notes = Vec::new();
    for item in items {
        let Some(text) = item.as_str() else {
            continue;
        };
        let text = text.trim();
        if text.is_empty() || text.chars().count() > MAX_TEXT_CHARS {
            continue;
        }
        notes.push(text.to_owned());
        if notes.len() == MAX_FLAVOR_NOTES {
            break;
        }
    }
    notes
}
