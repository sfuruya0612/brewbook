//! 地図の設定の API (FR-22、ADR-0019)。
//!
//! 経路は `GET /api/maps/config` とし、Maps Embed API の API キーを返す。
//! キーは地図と住所の補完で同じ 1 つを使い (`GOOGLE_MAPS_API_KEY`)、住所の補完は Worker が
//! アプリのオリジンの `Referer` を付けて呼ぶため、ブラウザと同じリファラの制限で通る。
//! キーは iframe の URL に載ってブラウザに出るため、リファラの制限で保護する (ADR-0019)。
//! 未設定のときは null を返し、画面は地図を出さない (キーの無いローカルや CI でもフォームは動く)。

use brew_book_core::maps::MapsConfigResponse;
use worker::{Env, Response, Result};

use crate::auth::{self, session::Session};
use crate::respond;

/// 地図と住所の補完の API キーの Secret の名前 (ADR-0019)。
pub const API_KEY_SECRET: &str = "GOOGLE_MAPS_API_KEY";

/// 地図の設定を返す。認証が必要 (FR-22)。
pub async fn config(env: &Env, _session: &Session) -> Result<Response> {
    let key = auth::var_or(env, API_KEY_SECRET, "")?;
    let embed_api_key = (!key.is_empty()).then_some(key);
    respond::json(&MapsConfigResponse { embed_api_key })
}
