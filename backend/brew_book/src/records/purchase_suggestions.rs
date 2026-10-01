//! 写真から購入と商品の項目を推測する API (FR-19、ADR-0016)。
//!
//! 入力はクライアントが変換した後の JPEG の本体とする (Content-Type、本体が空でないこと、
//! 5 MB の上限を検証し、合わない入力は AI を呼ばずに 400 を返す)。推測は Workers AI の
//! vision モデルを、Worker の AI バインディング (`AI`) から呼んで行う。
//!
//! プロンプトの組み立てと応答の解析は `brew_book_core::suggestion` が持ち、ここは経路の処理
//! (入力の読み取り、AI の呼び出し、応答の組み立て) だけを行う。
//! 推測の入力 (写真) と出力はログに出さない (FR-19、PRD のセキュリティ)。
//! AI の呼び出しの失敗 (無料枠の超過、タイムアウト、モデルのエラー) は、理由を区別せず
//! 500 を返す。クライアントは失敗を表示し、手入力で続けられる。

use brew_book_core::error::ErrorCode;
use brew_book_core::photo;
use brew_book_core::suggestion;
use worker::{console_error, Env, Request, Response, Result};

use crate::auth::session::Session;
use crate::auth::var_or;
use crate::respond;

/// AI バインディングの名前 (ADR-0016)。
pub const BINDING: &str = "AI";
/// モデル名の vars の名前。環境ごとに差し替えられるようにする (ADR-0016)。
pub const MODEL_VAR: &str = "AI_SUGGEST_MODEL";
/// モデル名の既定値。2026-09-30 の実写真での比較で決めた (issue 0034)。
/// 名前の抽出の精度、所要 (約 4 秒)、1 回あたりの消費 (約 62 Neurons) の均衡から選んだ (ADR-0016)。
pub const DEFAULT_MODEL: &str = "@cf/meta/llama-4-scout-17b-16e-instruct";
/// 出力の上限の tokens。応答の JSON は 200 tokens 程度で、長い出力を抑える (ADR-0016)。
const MAX_TOKENS: u32 = 512;
/// 推論の temperature。抽出のため 0 にする。
const TEMPERATURE: f32 = 0.0;

/// `POST /api/purchase-suggestions` の入力 (Workers AI の chat 形式)。
///
/// 画像は `messages` の中の `image_url` の `data:` URL で渡す (ADR-0016)。
#[derive(serde::Serialize)]
struct AiInput<'a> {
    messages: [AiMessage<'a>; 1],
    max_tokens: u32,
    temperature: f32,
}

/// 1 つのメッセージ。役割は常に `user` にする。
#[derive(serde::Serialize)]
struct AiMessage<'a> {
    role: &'static str,
    content: [AiContent<'a>; 2],
}

/// メッセージの中身。プロンプトの文と画像の組にする。
#[derive(serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AiContent<'a> {
    /// プロンプトの文。
    Text { text: &'a str },
    /// 画像の `data:` URL。
    ImageUrl { image_url: AiImageUrl },
}

/// 画像の `data:` URL。
#[derive(serde::Serialize)]
struct AiImageUrl {
    url: String,
}

/// 写真から購入と商品の項目を推測する。認証が必要 (FR-19)。
pub async fn create(req: &mut Request, env: &Env, _session: &Session) -> Result<Response> {
    // Content-Type とサイズを先に検証し、合わない入力では AI を呼ばない (FR-19)。
    if !is_jpeg(req.headers().get("Content-Type")?.as_deref()) {
        return Ok(super::invalid_input("the content type must be image/jpeg"));
    }
    let bytes = req.bytes().await?;
    // 本体が空の入力は写真として成立しないため、AI を呼ばずに 400 にする (FR-19)。
    if bytes.is_empty() {
        return Ok(super::invalid_input("the photo must not be empty"));
    }
    if bytes.len() > photo::MAX_BYTES as usize {
        return Ok(super::invalid_input(
            "the photo must not exceed 5000000 bytes",
        ));
    }
    let model = var_or(env, MODEL_VAR, DEFAULT_MODEL)?;
    let model = if model.trim().is_empty() {
        DEFAULT_MODEL.to_owned()
    } else {
        model
    };
    let input = AiInput {
        messages: [AiMessage {
            role: "user",
            content: [
                AiContent::Text {
                    text: suggestion::prompt(),
                },
                AiContent::ImageUrl {
                    image_url: AiImageUrl {
                        url: suggestion::image_data_url(&bytes),
                    },
                },
            ],
        }],
        max_tokens: MAX_TOKENS,
        temperature: TEMPERATURE,
    };
    let ai = match env.ai(BINDING) {
        Ok(ai) => ai,
        Err(error) => {
            console_error!("the AI binding is not available: {error}");
            return Ok(respond::error(ErrorCode::Internal, "the suggestion failed"));
        }
    };
    let response = match ai.run::<_, serde_json::Value>(&model, input).await {
        Ok(response) => response,
        Err(error) => {
            // 失敗の理由 (無料枠の超過、タイムアウト、モデルのエラー) は区別しない (FR-19)。
            // 入力 (写真) と出力はログに出さない (PRD のセキュリティ)。
            console_error!("the inference of the purchase suggestion failed: {error}");
            return Ok(respond::error(ErrorCode::Internal, "the suggestion failed"));
        }
    };
    // 応答はモデルにより形が異なるため、出力のテキストを取り出してから検証する (FR-19)。
    // 形式に合わない項目は null に落とす (FR-19)。
    respond::json(&suggestion::parse_response(&suggestion::output_text(
        &response,
    )))
}

/// Content-Type が `image/jpeg` か。パラメータ (`; charset=...`) は無視し、大文字と小文字を
/// 区別しない。無い場合と他の形式は拒否する (FR-19)。
fn is_jpeg(content_type: Option<&str>) -> bool {
    content_type.is_some_and(|value| {
        value
            .split(';')
            .next()
            .is_some_and(|value| value.trim().eq_ignore_ascii_case(photo::CONTENT_TYPE))
    })
}
