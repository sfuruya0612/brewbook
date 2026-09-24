//! 全記録のエクスポート (FR-14)。
//!
//! ADR-0006 の利用者データの 6 テーブル (shops、products、flavor_tags、product_flavor_tags、
//! purchases、brews) の、アーカイブ済みを含む全行と全列を 1 つの JSON にまとめる。
//! パスキー、セッション、チャレンジ、登録用トークンは含めない。
//! 購入の行は `photo_key` に加えて写真取得 API のパスを `photo_path` として持ち、写真の実体と
//! 署名付き GET URL は含めない (ADR-0003)。
//! エクスポート日時などの追加のメタデータは入れない (復元テストの比較対象をテーブルの行と列
//! だけにするため)。
//! 応答はダウンロード用の JSON とし、`Content-Disposition: attachment` を付ける。

use coffee_log_core::query::{self, Statement};
use serde::{Deserialize, Serialize};
use worker::d1::D1Database;
use worker::{Env, Response, Result};

use crate::auth::session::Session;
use crate::db;
use crate::respond;

/// ダウンロードのファイル名。
const FILE_NAME: &str = "coffee-log-export.json";

/// 店の行。列は shops テーブルと同じ。
#[derive(Debug, Serialize, Deserialize)]
struct ShopRow {
    id: String,
    user_id: String,
    name: String,
    address: Option<String>,
    created_at: String,
    updated_at: String,
    archived_at: Option<String>,
}

/// 商品の行。列は products テーブルと同じ。
#[derive(Debug, Serialize, Deserialize)]
struct ProductRow {
    id: String,
    user_id: String,
    name: String,
    producer: Option<String>,
    origin: Option<String>,
    region: Option<String>,
    process: Option<String>,
    variety: Option<String>,
    created_at: String,
    updated_at: String,
    archived_at: Option<String>,
}

/// Flavor Notes のタグの行。列は flavor_tags テーブルと同じ。
#[derive(Debug, Serialize, Deserialize)]
struct FlavorTagRow {
    id: String,
    user_id: String,
    name: String,
}

/// 商品と Flavor Notes のタグの対応の行。列は product_flavor_tags テーブルと同じ。
#[derive(Debug, Serialize, Deserialize)]
struct ProductFlavorTagRow {
    user_id: String,
    product_id: String,
    tag_id: String,
}

/// 購入の行。列は purchases テーブルと同じ。
#[derive(Debug, Serialize, Deserialize)]
struct PurchaseRow {
    id: String,
    user_id: String,
    product_id: String,
    shop_id: Option<String>,
    purchased_on: String,
    roast: Option<String>,
    roast_date: Option<String>,
    price_amount: Option<i64>,
    price_currency: Option<String>,
    weight_grams: Option<i64>,
    photo_key: Option<String>,
    created_at: String,
    updated_at: String,
    archived_at: Option<String>,
}

/// エクスポートの購入の行。テーブルの列に写真取得 API のパスを加える (ADR-0003)。
#[derive(Debug, Serialize)]
struct PurchaseExportRow {
    #[serde(flatten)]
    row: PurchaseRow,
    /// 写真取得 API のパス。写真の有無に関わらず含める。
    photo_path: String,
}

impl PurchaseExportRow {
    /// テーブルの行から組み立てる。
    fn new(row: PurchaseRow) -> Self {
        let photo_path = photo_path(&row.id);
        Self { row, photo_path }
    }
}

/// 抽出の行。列は brews テーブルと同じ。
#[derive(Debug, Serialize, Deserialize)]
struct BrewRow {
    id: String,
    user_id: String,
    purchase_id: String,
    brewed_at: String,
    dose_grams: Option<f64>,
    water_grams: Option<f64>,
    water_temp_c: Option<f64>,
    brew_time_seconds: Option<i64>,
    method: Option<String>,
    grind_setting: Option<String>,
    rating: Option<i64>,
    notes: Option<String>,
    created_at: String,
    updated_at: String,
    archived_at: Option<String>,
}

/// `GET /api/export` の応答。トップレベルに 6 テーブルの名前を持つ配列を並べる (FR-14)。
#[derive(Debug, Serialize)]
struct ExportResponse {
    shops: Vec<ShopRow>,
    products: Vec<ProductRow>,
    flavor_tags: Vec<FlavorTagRow>,
    product_flavor_tags: Vec<ProductFlavorTagRow>,
    purchases: Vec<PurchaseExportRow>,
    brews: Vec<BrewRow>,
}

/// 利用者の全記録を JSON で返す。認証が必要 (FR-14)。
pub async fn get(env: &Env, session: &Session) -> Result<Response> {
    let d1 = db::database(env)?;
    let user_id = &session.user_id;
    // 全行を読むため、応答は利用者の記録の大きさだけメモリに載る。想定規模 (抽出 30,000 件) で
    // 収まることは結合テスト (wrangler_records_scale) が確認する。
    let shops = rows::<ShopRow>(
        &d1,
        &query::export_rows(query::SHOPS_TABLE, query::SHOP_COLUMNS, "id ASC", user_id),
    )
    .await?;
    let products = rows::<ProductRow>(
        &d1,
        &query::export_rows(
            query::PRODUCTS_TABLE,
            query::PRODUCT_COLUMNS,
            "id ASC",
            user_id,
        ),
    )
    .await?;
    let flavor_tags = rows::<FlavorTagRow>(
        &d1,
        &query::export_rows(
            query::FLAVOR_TAGS_TABLE,
            query::FLAVOR_TAG_COLUMNS,
            "id ASC",
            user_id,
        ),
    )
    .await?;
    let product_flavor_tags = rows::<ProductFlavorTagRow>(
        &d1,
        &query::export_rows(
            query::PRODUCT_FLAVOR_TAGS_TABLE,
            query::PRODUCT_FLAVOR_TAG_COLUMNS,
            "product_id ASC, tag_id ASC",
            user_id,
        ),
    )
    .await?;
    let purchases = rows::<PurchaseRow>(
        &d1,
        &query::export_rows(
            query::PURCHASES_TABLE,
            query::PURCHASE_COLUMNS,
            "id ASC",
            user_id,
        ),
    )
    .await?;
    let brews = rows::<BrewRow>(
        &d1,
        &query::export_rows(query::BREWS_TABLE, query::BREW_COLUMNS, "id ASC", user_id),
    )
    .await?;
    respond::json_attachment(
        &ExportResponse {
            shops,
            products,
            flavor_tags,
            product_flavor_tags,
            purchases: purchases.into_iter().map(PurchaseExportRow::new).collect(),
            brews,
        },
        FILE_NAME,
    )
}

/// エクスポートの 1 つのテーブルの全行を引く。
async fn rows<T: serde::de::DeserializeOwned>(
    d1: &D1Database,
    statement: &Statement,
) -> Result<Vec<T>> {
    db::prepared(d1, statement)?.all().await?.results::<T>()
}

/// 写真取得 API のパス (FR-10、ADR-0003)。
fn photo_path(purchase_id: &str) -> String {
    format!("/api/purchases/{purchase_id}/photo")
}
