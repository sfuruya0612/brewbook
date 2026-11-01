//! 購入の豆の情報から、味と特徴を推定する API (FR-24、ADR-0023)。
//!
//! 経路は `POST /api/purchases/:id/taste-profile?lang=<ja|en>` とする。購入が存在するかを
//! 確かめ、認証した利用者の購入だけを対象にする (他の利用者の ID は 404)。
//! 推定は Workers AI の言語モデルを、Worker の AI バインディング (`AI`) から呼んで行う。
//!
//! プロンプトの組み立てと応答の解析は `brew_book_core::taste` が持ち、ここは経路の処理
//! (入力の読み取り、AI の呼び出し、応答の組み立て) だけを行う。
//! 推定の入力 (豆の情報) と出力はログに出さない (FR-24、PRD のセキュリティ)。
//! AI の呼び出しの失敗 (無料枠の超過、タイムアウト、モデルのエラー) は、理由を区別せず
//! 500 を返す。クライアントは失敗を表示し、再試行できる。
//!
//! 推定は保存する。保存先は購入の `taste_profile` で、JSON の文字列として上書きする
//! (FR-24、ADR-0023)。購入を削除すると推定も消え、エクスポートとアカウント削除の対象にも
//! 購入の列として含まれる。外部 (Cloudflare 以外) の AI サービスや Web の検索には送らない。

use brew_book_core::error::ErrorCode;
use brew_book_core::query;
use brew_book_core::suggestion;
use brew_book_core::taste::{self, BeanInfo};
use worker::{console_error, Env, Request, Response, Result};

use super::purchase_suggestions::BINDING;
use super::purchases::PurchaseJoinRow;
use super::{attach_flavor_notes, invalid_input, not_found};
use crate::auth::session::Session;
use crate::auth::var_or;
use crate::db;
use crate::respond;

/// モデル名の vars の名前。環境ごとに差し替えられるようにする (ADR-0023)。
pub const MODEL_VAR: &str = "AI_TASTE_MODEL";
/// モデル名の既定値。写真からの推測と同じモデルを使い、AI バインディングの設定を増やさない
/// (ADR-0016、ADR-0023)。
pub const DEFAULT_MODEL: &str = "@cf/meta/llama-4-scout-17b-16e-instruct";
/// 出力の上限の tokens。味と特徴と淹れ方と産地の説明の JSON は 1000 tokens 程度で、
/// 長い出力を抑える。
const MAX_TOKENS: u32 = 1536;
/// 推定の temperature。豆の情報に基づく安定した説明のため 0.2 にする。
const TEMPERATURE: f32 = 0.2;

/// Workers AI の chat 形式の入力 (テキストだけ)。
#[derive(serde::Serialize)]
struct AiInput {
    messages: [AiMessage; 1],
    max_tokens: u32,
    temperature: f32,
}

/// 1 つのメッセージ。役割は常に `user` にする。
#[derive(serde::Serialize)]
struct AiMessage {
    role: &'static str,
    content: String,
}

/// 購入の豆の情報から、味と特徴を推定する。認証が必要 (FR-24)。
pub async fn profile(
    req: &Request,
    env: &Env,
    session: &Session,
    id: Option<&str>,
) -> Result<Response> {
    let Some(id) = id else {
        return Ok(not_found("the purchase does not exist"));
    };
    // 言語は AI を呼ぶ前に検証し、合わない入力では購入を読まない (FR-24)。
    let language = match language(req) {
        Ok(language) => language,
        Err(response) => return Ok(response),
    };
    let d1 = db::database(env)?;
    let statement = query::purchase_find(&session.user_id, id);
    let row: Option<PurchaseJoinRow> = db::prepared(&d1, &statement)?.first(None).await?;
    let Some(row) = row else {
        return Ok(not_found("the purchase does not exist"));
    };
    let mut purchase = row.into_response();
    let mut purchases = [&mut purchase];
    attach_flavor_notes(&d1, &session.user_id, &mut purchases).await?;
    let info = BeanInfo {
        name: &purchase.name,
        producer: purchase.producer.as_deref(),
        origin: purchase.origin.as_deref(),
        region: purchase.region.as_deref(),
        process: purchase.process.as_deref(),
        variety: purchase.variety.as_deref(),
        flavor_notes: &purchase.flavor_notes,
        roast: purchase.roast.as_deref(),
    };
    let model = var_or(env, MODEL_VAR, DEFAULT_MODEL)?;
    let model = if model.trim().is_empty() {
        DEFAULT_MODEL.to_owned()
    } else {
        model
    };
    let input = AiInput {
        messages: [AiMessage {
            role: "user",
            content: taste::prompt(&info, language),
        }],
        max_tokens: MAX_TOKENS,
        temperature: TEMPERATURE,
    };
    let ai = match env.ai(BINDING) {
        Ok(ai) => ai,
        Err(error) => {
            console_error!("the AI binding is not available: {error}");
            return Ok(respond::error(
                ErrorCode::Internal,
                "the taste profile failed",
            ));
        }
    };
    let response = match ai.run::<_, serde_json::Value>(&model, input).await {
        Ok(response) => response,
        Err(error) => {
            // 失敗の理由 (無料枠の超過、タイムアウト、モデルのエラー) は区別しない (FR-24)。
            // 入力 (豆の情報) と出力はログに出さない (PRD のセキュリティ)。
            console_error!("the inference of the taste profile failed: {error}");
            return Ok(respond::error(
                ErrorCode::Internal,
                "the taste profile failed",
            ));
        }
    };
    // 応答はモデルにより形が異なるため、出力のテキストを取り出してから検証する (FR-24)。
    let profile = taste::parse_response(&suggestion::output_text(&response));
    if !profile.has_content() {
        console_error!("the taste profile has no valid content");
        return Ok(respond::error(
            ErrorCode::Internal,
            "the taste profile failed",
        ));
    }
    // 保存に成功した推定だけを返す。保存に失敗したときは、前の推定を残したまま 500 にする。
    let text = match serde_json::to_string(&profile) {
        Ok(text) => text,
        Err(error) => {
            console_error!("the taste profile is not serializable: {error}");
            return Ok(respond::error(
                ErrorCode::Internal,
                "the taste profile failed",
            ));
        }
    };
    let statement = match query::purchase_set_taste_profile(id, &session.user_id, &text) {
        Ok(statement) => statement,
        Err(error) => {
            console_error!("the taste profile query is invalid: {}", error.message());
            return Ok(respond::error(
                ErrorCode::Internal,
                "the taste profile failed",
            ));
        }
    };
    if let Err(error) = db::prepared(&d1, &statement)?.run().await {
        // 入力 (豆の情報) と出力はログに出さない (PRD のセキュリティ)。
        console_error!("saving the taste profile failed: {error}");
        return Ok(respond::error(
            ErrorCode::Internal,
            "the taste profile failed",
        ));
    }
    respond::json(&profile)
}

/// クエリパラメータの `lang` を読み、検証する (FR-24)。
///
/// 無い場合と複数回の指定は 400 にする。入力の言語は日本語と英語の 2 つだけとする (FR-16)。
fn language(req: &Request) -> std::result::Result<&'static str, Response> {
    let url = match req.url() {
        Ok(url) => url,
        Err(error) => {
            console_error!("the request URL is not readable: {error}");
            return Err(super::internal_error());
        }
    };
    let mut lang = None;
    for (name, value) in url.query_pairs() {
        if name.as_ref() == "lang" {
            if lang.is_some() {
                return Err(invalid_input("lang must be a single string"));
            }
            lang = Some(value.into_owned());
        }
    }
    let lang = lang.unwrap_or_default();
    taste::validate_language(&lang).map_err(|error| invalid_input(error.message()))
}
