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

use brew_book_core::query;
use serde::{Deserialize, Serialize};
use worker::d1::D1Result;
use worker::{console_error, Env, Response, Result};

use crate::auth::session::Session;
use crate::db;
use crate::respond;

/// ダウンロードのファイル名。
const FILE_NAME: &str = "brewbook-export.json";

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
    // 6 つの SELECT を 1 つの batch (1 トランザクション) で実行し、単一の時点の記録を返す
    // (別々に読むと、親の無い抽出を含む復元できない JSON になり得る)。
    // 全行を読むため、応答は利用者の記録の大きさだけメモリに載る。想定規模 (抽出 30,000 件) で
    // 収まることは結合テスト (wrangler_records_scale) が確認する。
    let statements = vec![
        db::prepared(
            &d1,
            &query::export_rows(query::SHOPS_TABLE, query::SHOP_COLUMNS, "id ASC", user_id),
        )?,
        db::prepared(
            &d1,
            &query::export_rows(
                query::PRODUCTS_TABLE,
                query::PRODUCT_COLUMNS,
                "id ASC",
                user_id,
            ),
        )?,
        db::prepared(
            &d1,
            &query::export_rows(
                query::FLAVOR_TAGS_TABLE,
                query::FLAVOR_TAG_COLUMNS,
                "id ASC",
                user_id,
            ),
        )?,
        db::prepared(
            &d1,
            &query::export_rows(
                query::PRODUCT_FLAVOR_TAGS_TABLE,
                query::PRODUCT_FLAVOR_TAG_COLUMNS,
                "product_id ASC, tag_id ASC",
                user_id,
            ),
        )?,
        db::prepared(
            &d1,
            &query::export_rows(
                query::PURCHASES_TABLE,
                query::PURCHASE_COLUMNS,
                "id ASC",
                user_id,
            ),
        )?,
        db::prepared(
            &d1,
            &query::export_rows(query::BREWS_TABLE, query::BREW_COLUMNS, "id ASC", user_id),
        )?,
    ];
    let mut results = d1.batch(statements).await?.into_iter();
    let shops: Vec<ShopRow> = take_rows(&mut results, query::SHOPS_TABLE)?;
    let products: Vec<ProductRow> = take_rows(&mut results, query::PRODUCTS_TABLE)?;
    let flavor_tags: Vec<FlavorTagRow> = take_rows(&mut results, query::FLAVOR_TAGS_TABLE)?;
    let product_flavor_tags: Vec<ProductFlavorTagRow> =
        take_rows(&mut results, query::PRODUCT_FLAVOR_TAGS_TABLE)?;
    let purchases: Vec<PurchaseRow> = take_rows(&mut results, query::PURCHASES_TABLE)?;
    let brews: Vec<BrewRow> = take_rows(&mut results, query::BREWS_TABLE)?;
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

/// batch の結果から 1 つのテーブルの行を取り出す。結果の並びは `statements` と同じである。
fn take_rows<T: serde::de::DeserializeOwned>(
    results: &mut std::vec::IntoIter<D1Result>,
    table: &str,
) -> Result<Vec<T>> {
    let result = results.next().ok_or_else(|| {
        worker::Error::RustError(format!("the batch result for {table} is missing"))
    })?;
    if !result.success() {
        let error = result.error().unwrap_or_default();
        console_error!("the export query for {table} failed: {error}");
        return Err(worker::Error::RustError(
            "the export query failed".to_owned(),
        ));
    }
    result.results()
}

/// 写真取得 API のパス (FR-10、ADR-0003)。
fn photo_path(purchase_id: &str) -> String {
    format!("/api/purchases/{purchase_id}/photo")
}
