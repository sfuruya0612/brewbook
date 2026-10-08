//! 店の地図と住所の補完 (FR-22、ADR-0019) の Google Maps Platform の連携。
//!
//! 住所の補完の検索の入力の検証と、Places API (New) の Text Search の応答の解析を持つ。
//! Google の呼び出しは Worker 側 (`brew_book::records::place_search`) が行う。
//! ここは wasm に依存せず、ネイティブのテストで検証する (ADR-0001)。
//!
//! 応答の解析は、名前 (`displayName.text`) と住所 (`formattedAddress`) の両方がある場所だけを
//! 候補にし、どちらかが欠けた場所は除く。`places` が無い応答は候補 0 件とする。
//! 応答が JSON として読めないときと、オブジェクトでないとき、`places` が配列でないときは
//! 誤りにする (500 にする)。

use serde::{Deserialize, Serialize};

/// 検索の店名の上限 (文字数)。
pub const MAX_QUERY_CHARS: usize = 256;

/// 候補の上限 (件数)。Places API の `pageSize` にも同じ値を渡す。
pub const MAX_CANDIDATES: usize = 5;

/// 住所の補完が受け付ける言語 (`languageCode` に渡す値)。UI の言語と同じ 2 つにする (FR-16)。
pub const SUPPORTED_LANGUAGES: [&str; 2] = ["ja", "en"];

/// 住所の検索の候補 1 件 (FR-22)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    /// 場所の名前。
    pub name: String,
    /// 住所。
    pub address: String,
}

/// `GET /api/place-search` の応答 (FR-22)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResponse {
    /// 候補。最大 [`MAX_CANDIDATES`] 件で、無いときは空。
    pub candidates: Vec<Candidate>,
}

/// `GET /api/maps/config` の応答 (FR-22)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapsConfigResponse {
    /// 地図の埋め込み (Maps Embed API) の API キー。未設定のときは None (画面は地図を出さない)。
    pub embed_api_key: Option<String>,
}

/// 住所の検索の入力の検証の誤り。応答は 400 にする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchInputError {
    /// 店名が前後の空白を除いて空。
    EmptyQuery,
    /// 店名が [`MAX_QUERY_CHARS`] 文字を超える。
    QueryTooLong,
    /// 言語が [`SUPPORTED_LANGUAGES`] のどれでもない。
    UnsupportedLanguage,
}

impl SearchInputError {
    /// 応答に載せる英語のメッセージ。
    pub fn message(self) -> &'static str {
        match self {
            Self::EmptyQuery => "the query must not be empty",
            Self::QueryTooLong => "the query must be at most 256 characters",
            Self::UnsupportedLanguage => "the language must be ja or en",
        }
    }
}

/// 住所の検索の入力を検証し、前後の空白を除いた店名と、Google に渡す言語コードを返す (FR-22)。
pub fn validate_search_input(
    query: &str,
    lang: &str,
) -> Result<(String, &'static str), SearchInputError> {
    let query = query.trim();
    if query.is_empty() {
        return Err(SearchInputError::EmptyQuery);
    }
    if query.chars().count() > MAX_QUERY_CHARS {
        return Err(SearchInputError::QueryTooLong);
    }
    let lang = SUPPORTED_LANGUAGES
        .iter()
        .find(|supported| **supported == lang)
        .copied()
        .ok_or(SearchInputError::UnsupportedLanguage)?;
    Ok((query.to_owned(), lang))
}

/// Places API (New) の Text Search のリクエストの本体 (FR-22)。
///
/// 店名を `textQuery`、言語を `languageCode`、候補の上限を `pageSize` に渡す。
/// フィールドマスクは経路の処理がヘッダーで渡す (応答で返す項目はここでは決めない)。
pub fn search_request_body(query: &str, lang: &str) -> String {
    serde_json::json!({
        "textQuery": query,
        "pageSize": MAX_CANDIDATES,
        "languageCode": lang,
    })
    .to_string()
}

/// Google の応答の解析の誤り。応答は 500 にする。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    /// JSON として読めない。
    InvalidJson,
    /// 応答がオブジェクトでない、または `places` が配列でない。
    InvalidShape,
}

impl ParseError {
    /// ログに載せる英語のメッセージ。
    pub fn message(self) -> &'static str {
        match self {
            Self::InvalidJson => "the response is not valid JSON",
            Self::InvalidShape => "the response has an unexpected shape",
        }
    }
}

/// Places API (New) の Text Search の応答を検証し、候補にする (FR-22)。
///
/// 名前と住所の両方を持つ場所だけを [`MAX_CANDIDATES`] 件まで取り出す。
/// 名前と住所は前後の空白を除き、除いた結果が空になる場所は除く。
/// `places` が無い応答は候補 0 件にする (検索の結果が無いときの応答)。
pub fn parse_search_response(body: &str) -> Result<Vec<Candidate>, ParseError> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|_| ParseError::InvalidJson)?;
    let Some(object) = value.as_object() else {
        return Err(ParseError::InvalidShape);
    };
    let places = match object.get("places") {
        None => return Ok(Vec::new()),
        Some(serde_json::Value::Array(places)) => places,
        Some(_) => return Err(ParseError::InvalidShape),
    };
    let mut candidates = Vec::new();
    for place in places {
        let name = place
            .get("displayName")
            .and_then(|display| display.get("text"))
            .and_then(|text| text.as_str())
            .map(str::trim);
        let address = place
            .get("formattedAddress")
            .and_then(|address| address.as_str())
            .map(str::trim);
        let (Some(name), Some(address)) = (name, address) else {
            continue;
        };
        if name.is_empty() || address.is_empty() {
            continue;
        }
        candidates.push(Candidate {
            name: name.to_owned(),
            address: address.to_owned(),
        });
        if candidates.len() == MAX_CANDIDATES {
            break;
        }
    }
    Ok(candidates)
}
