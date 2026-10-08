//! 地図の設定の API (FR-22、ADR-0019)。
//!
//! 経路は `GET /api/maps/config` とし、Maps Embed API の API キーを返す。
//! キーは iframe の URL に載ってブラウザに出るため、Google Cloud 側のリファラの制限で
//! 保護する (ADR-0019)。未設定のときは null を返し、画面は地図を出さない
//! (キーの無いローカルや CI でもフォームは動く)。

use brew_book_core::maps::MapsConfigResponse;
use worker::{Env, Response, Result};

use crate::auth::{self, session::Session};
use crate::respond;

/// 地図の API キーの Secret の名前 (ADR-0019)。
pub const EMBED_API_KEY_SECRET: &str = "GOOGLE_MAPS_EMBED_API_KEY";

/// 地図の設定を返す。認証が必要 (FR-22)。
pub async fn config(env: &Env, _session: &Session) -> Result<Response> {
    let key = auth::var_or(env, EMBED_API_KEY_SECRET, "")?;
    let embed_api_key = (!key.is_empty()).then_some(key);
    respond::json(&MapsConfigResponse { embed_api_key })
}
