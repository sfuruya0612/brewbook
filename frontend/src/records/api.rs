//! 記録の API (FR-6 から FR-13) の呼び出し。
//!
//! 経路の組み立てと、応答の型への変換だけを行い、状態は持たない。一覧はカーソル方式で、
//! 1 ページの件数は既定の 50 件とする (PRD の性能)。Flutter の
//! `frontend/lib/api/records_api.dart` と同じ経路を呼ぶ。

use crate::api::{ApiCallError, ApiClient};
use serde_json::json;

use super::inputs::{BrewInput, ProductInput, PurchaseInput, ShopInput};
use super::models::{
    items_field, next_cursor_field, optional_string, Brew, DeleteImpact, PhotoUploadTarget,
    PlaceCandidate, Product, Purchase, PurchaseSuggestion, RecordPage, Shop,
};
use super::RecordError;

/// サジェスト (FR-13) の対象の項目名。値は API の経路の名前と同じにする。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SuggestionTarget {
    /// 商品の生産者。
    Producer,
    /// 商品の生産国。
    Origin,
    /// 商品の地域。
    Region,
    /// 商品の精製方法。
    Process,
    /// 商品の品種。
    Variety,
    /// 購入の焙煎度。
    Roast,
    /// 抽出の抽出方法。
    Method,
    /// 抽出の挽き目。
    GrindSetting,
}

impl SuggestionTarget {
    /// 経路に使う名前。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Producer => "producer",
            Self::Origin => "origin",
            Self::Region => "region",
            Self::Process => "process",
            Self::Variety => "variety",
            Self::Roast => "roast",
            Self::Method => "method",
            Self::GrindSetting => "grind_setting",
        }
    }

    /// 全ての対象 (API が受け付ける 8 つ)。
    pub const ALL: [SuggestionTarget; 8] = [
        SuggestionTarget::Producer,
        SuggestionTarget::Origin,
        SuggestionTarget::Region,
        SuggestionTarget::Process,
        SuggestionTarget::Variety,
        SuggestionTarget::Roast,
        SuggestionTarget::Method,
        SuggestionTarget::GrindSetting,
    ];
}

/// 一覧の 1 ページの件数 (API の既定と同じ。PRD の性能)。
pub const PAGE_SIZE: u32 = 50;

/// 一覧の並び順の方向 (FR-20)。`order` の値。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SortOrder {
    /// 降順 (既定)。
    #[default]
    Desc,
    /// 昇順。
    Asc,
}

impl SortOrder {
    /// クエリパラメータに送る名前。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Desc => "desc",
            Self::Asc => "asc",
        }
    }

    /// 切り替えたときの反対の方向 (昇順/降順の切り替え。FR-20)。
    pub fn toggled(self) -> Self {
        match self {
            Self::Desc => Self::Asc,
            Self::Asc => Self::Desc,
        }
    }
}

/// 一覧の並び順と絞り込みの条件 (FR-20、FR-21)。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ListOptions {
    /// 並び順のキー (`sort` の値)。空のときは API の既定に任せる (選択のシート)。
    pub sort: &'static str,
    /// 並び順の方向。
    pub order: SortOrder,
    /// お気に入りだけに絞るか。
    pub favorite_only: bool,
}

impl ListOptions {
    /// 並び順のキーを指定して、降順でお気に入りの絞り込みなしの条件を作る。
    pub fn new(sort: &'static str) -> Self {
        Self {
            sort,
            order: SortOrder::Desc,
            favorite_only: false,
        }
    }
}

/// 記録の API を [`ApiClient`] で呼ぶ。
#[derive(Clone)]
pub struct RecordsApi {
    api: ApiClient,
}

impl PartialEq for RecordsApi {
    /// 同じ API クライアントを指しているときだけ等しいとみなす。
    fn eq(&self, other: &Self) -> bool {
        self.api == other.api
    }
}

impl RecordsApi {
    /// API クライアントから作る。
    pub fn new(api: ApiClient) -> Self {
        Self { api }
    }

    /// 元の API クライアント。
    pub fn client(&self) -> &ApiClient {
        &self.api
    }

    /// 店の一覧を引く (FR-6、FR-20、FR-21)。
    pub async fn shops(
        &self,
        options: &ListOptions,
        cursor: Option<&str>,
    ) -> Result<RecordPage<Shop>, RecordError> {
        let json = self
            .api
            .get_json(&list_path("/shops", *options, cursor, None))
            .await?;
        Ok(RecordPage {
            items: items_field(&json, "shops", Shop::from_json)?,
            next_cursor: next_cursor_field(&json)?,
        })
    }

    /// 店のお気に入りを付け外しする (FR-21)。更新後の店を返す。
    pub async fn set_shop_favorite(&self, id: &str, favorite: bool) -> Result<Shop, RecordError> {
        favorite_record(&self.api, "shops", id, favorite, Shop::from_json).await
    }

    /// 店を削除する (0056)。
    pub async fn delete_shop(&self, id: &str) -> Result<(), RecordError> {
        self.api.delete_json(&format!("/shops/{id}")).await?;
        Ok(())
    }

    /// 店の削除で店の指定が外れる購入の件数を引く (0056)。
    pub async fn shop_delete_impact(&self, id: &str) -> Result<DeleteImpact, RecordError> {
        DeleteImpact::from_json(
            &self
                .api
                .get_json(&format!("/shops/{id}/delete-impact"))
                .await?,
        )
    }

    /// 店を 1 件引く (FR-6)。
    pub async fn shop(&self, id: &str) -> Result<Shop, RecordError> {
        Shop::from_json(&self.api.get_json(&format!("/shops/{id}")).await?)
    }

    /// 店を登録する (FR-6)。
    pub async fn create_shop(&self, input: &ShopInput) -> Result<Shop, RecordError> {
        Shop::from_json(&self.api.post_json("/shops", &input.to_json()).await?)
    }

    /// 店を更新する (FR-6)。
    pub async fn update_shop(&self, id: &str, input: &ShopInput) -> Result<Shop, RecordError> {
        Shop::from_json(
            &self
                .api
                .patch_json(&format!("/shops/{id}"), &input.to_json())
                .await?,
        )
    }

    /// 商品の一覧を引く (FR-7、FR-8、FR-20、FR-21)。
    ///
    /// `name` を指定したときは、前後の空白を除いて大文字と小文字を区別しない名前の完全一致で
    /// 絞り込む (FR-19。写真からの推測で一致する商品を 1 リクエストで引くために使う)。
    pub async fn products(
        &self,
        options: &ListOptions,
        cursor: Option<&str>,
        name: Option<&str>,
    ) -> Result<RecordPage<Product>, RecordError> {
        let json = self
            .api
            .get_json(&list_path("/products", *options, cursor, name))
            .await?;
        Ok(RecordPage {
            items: items_field(&json, "products", Product::from_json)?,
            next_cursor: next_cursor_field(&json)?,
        })
    }

    /// 商品のお気に入りを付け外しする (FR-21)。更新後の商品を返す。
    pub async fn set_product_favorite(
        &self,
        id: &str,
        favorite: bool,
    ) -> Result<Product, RecordError> {
        favorite_record(&self.api, "products", id, favorite, Product::from_json).await
    }

    /// 商品を削除する (0056)。
    pub async fn delete_product(&self, id: &str) -> Result<(), RecordError> {
        self.api.delete_json(&format!("/products/{id}")).await?;
        Ok(())
    }

    /// 商品の削除で消える購入と抽出の件数を引く (0056)。
    pub async fn product_delete_impact(&self, id: &str) -> Result<DeleteImpact, RecordError> {
        DeleteImpact::from_json(
            &self
                .api
                .get_json(&format!("/products/{id}/delete-impact"))
                .await?,
        )
    }

    /// 商品を 1 件引く (FR-7)。
    pub async fn product(&self, id: &str) -> Result<Product, RecordError> {
        Product::from_json(&self.api.get_json(&format!("/products/{id}")).await?)
    }

    /// 商品を登録する (FR-7、FR-8)。
    pub async fn create_product(&self, input: &ProductInput) -> Result<Product, RecordError> {
        Product::from_json(&self.api.post_json("/products", &input.to_json()).await?)
    }

    /// 商品を更新する (FR-7、FR-8)。タグは入力した配列で置き換える (FR-8)。
    pub async fn update_product(
        &self,
        id: &str,
        input: &ProductInput,
    ) -> Result<Product, RecordError> {
        Product::from_json(
            &self
                .api
                .patch_json(&format!("/products/{id}"), &input.to_json())
                .await?,
        )
    }

    /// 購入の一覧を引く (FR-9、FR-20、FR-21)。
    pub async fn purchases(
        &self,
        options: &ListOptions,
        cursor: Option<&str>,
    ) -> Result<RecordPage<Purchase>, RecordError> {
        let json = self
            .api
            .get_json(&list_path("/purchases", *options, cursor, None))
            .await?;
        Ok(RecordPage {
            items: items_field(&json, "purchases", Purchase::from_json)?,
            next_cursor: next_cursor_field(&json)?,
        })
    }

    /// 購入のお気に入りを付け外しする (FR-21)。更新後の購入を返す。
    pub async fn set_purchase_favorite(
        &self,
        id: &str,
        favorite: bool,
    ) -> Result<Purchase, RecordError> {
        favorite_record(&self.api, "purchases", id, favorite, Purchase::from_json).await
    }

    /// 購入を削除する (0056)。
    pub async fn delete_purchase(&self, id: &str) -> Result<(), RecordError> {
        self.api.delete_json(&format!("/purchases/{id}")).await?;
        Ok(())
    }

    /// 購入の削除で消える抽出の件数を引く (0056)。
    pub async fn purchase_delete_impact(&self, id: &str) -> Result<DeleteImpact, RecordError> {
        DeleteImpact::from_json(
            &self
                .api
                .get_json(&format!("/purchases/{id}/delete-impact"))
                .await?,
        )
    }

    /// 購入を 1 件引く (FR-9)。
    pub async fn purchase(&self, id: &str) -> Result<Purchase, RecordError> {
        Purchase::from_json(&self.api.get_json(&format!("/purchases/{id}")).await?)
    }

    /// 購入を登録する (FR-9)。
    pub async fn create_purchase(&self, input: &PurchaseInput) -> Result<Purchase, RecordError> {
        Purchase::from_json(&self.api.post_json("/purchases", &input.to_json()).await?)
    }

    /// 購入を更新する (FR-9)。
    pub async fn update_purchase(
        &self,
        id: &str,
        input: &PurchaseInput,
    ) -> Result<Purchase, RecordError> {
        Purchase::from_json(
            &self
                .api
                .patch_json(&format!("/purchases/{id}"), &input.to_json())
                .await?,
        )
    }

    /// 抽出の一覧を引く (FR-11、FR-20、FR-21)。
    pub async fn brews(
        &self,
        options: &ListOptions,
        cursor: Option<&str>,
    ) -> Result<RecordPage<Brew>, RecordError> {
        let json = self
            .api
            .get_json(&list_path("/brews", *options, cursor, None))
            .await?;
        Ok(RecordPage {
            items: items_field(&json, "brews", Brew::from_json)?,
            next_cursor: next_cursor_field(&json)?,
        })
    }

    /// 抽出のお気に入りを付け外しする (FR-21)。更新後の抽出を返す。
    pub async fn set_brew_favorite(&self, id: &str, favorite: bool) -> Result<Brew, RecordError> {
        favorite_record(&self.api, "brews", id, favorite, Brew::from_json).await
    }

    /// 抽出を削除する (0056)。
    pub async fn delete_brew(&self, id: &str) -> Result<(), RecordError> {
        self.api.delete_json(&format!("/brews/{id}")).await?;
        Ok(())
    }

    /// 抽出を 1 件引く (FR-11)。
    pub async fn brew(&self, id: &str) -> Result<Brew, RecordError> {
        Brew::from_json(&self.api.get_json(&format!("/brews/{id}")).await?)
    }

    /// 抽出を登録する (FR-11)。
    pub async fn create_brew(&self, input: &BrewInput) -> Result<Brew, RecordError> {
        Brew::from_json(&self.api.post_json("/brews", &input.to_json()).await?)
    }

    /// 抽出を更新する (FR-11)。
    pub async fn update_brew(&self, id: &str, input: &BrewInput) -> Result<Brew, RecordError> {
        Brew::from_json(
            &self
                .api
                .patch_json(&format!("/brews/{id}"), &input.to_json())
                .await?,
        )
    }

    /// 写真のアップロード用 URL を発行する (FR-10)。変換後のサイズを申告する。
    pub async fn request_photo_upload_url(
        &self,
        purchase_id: &str,
        size: usize,
    ) -> Result<PhotoUploadTarget, RecordError> {
        let json = self
            .api
            .post_json(
                &format!("/purchases/{purchase_id}/photo/upload-url"),
                &json!({ "size": size }),
            )
            .await?;
        PhotoUploadTarget::from_json(&json)
    }

    /// 写真のアップロードの完了を通知する (FR-10)。紐づいた購入を返す。
    pub async fn complete_photo(
        &self,
        purchase_id: &str,
        key: &str,
        size: usize,
    ) -> Result<Purchase, RecordError> {
        let json = self
            .api
            .post_json(
                &format!("/purchases/{purchase_id}/photo"),
                &json!({ "key": key, "size": size }),
            )
            .await?;
        Purchase::from_json(&json)
    }

    /// 写真を削除する (FR-10)。紐づけを外した購入を返す。
    pub async fn delete_photo(&self, purchase_id: &str) -> Result<Purchase, RecordError> {
        Purchase::from_json(
            &self
                .api
                .delete_json(&format!("/purchases/{purchase_id}/photo"))
                .await?,
        )
    }

    /// 変換済みの写真から購入と商品の項目の推測を引く (FR-19)。
    ///
    /// 写真は変換済みの JPEG をそのまま送る。推測はどの記録も変更しない。
    /// AI の呼び出しに失敗したときは API が 500 を返し、[`RecordError::Api`] になる。
    pub async fn suggest_purchase(&self, image: &[u8]) -> Result<PurchaseSuggestion, RecordError> {
        let json = self
            .api
            .post_bytes("/purchase-suggestions", image.to_vec(), "image/jpeg")
            .await?;
        PurchaseSuggestion::from_json(&json)
    }

    /// 自由記述の項目の過去の入力値の候補を引く (FR-13)。
    ///
    /// 入力中の文字列で前方一致する候補を、API が最大 20 件返す。候補に無い値も入力できる。
    pub async fn suggestions(
        &self,
        target: SuggestionTarget,
        query: &str,
    ) -> Result<Vec<String>, RecordError> {
        let json = self
            .api
            .get_json(&format!(
                "/suggestions/{}?q={}",
                target.as_str(),
                encode_query(query)
            ))
            .await?;
        let Some(serde_json::Value::Array(values)) = json.get("values") else {
            return Err(super::models::format_error(
                "the values field must be an array",
            ));
        };
        values
            .iter()
            .map(|value| match value {
                serde_json::Value::String(value) => Ok(value.clone()),
                _ => Err(super::models::format_error(
                    "an item of values must be a string",
                )),
            })
            .collect()
    }

    /// 写真の取得の URL (FR-10)。写真は Backend が認証付きで返すため、この URL を表示に使う。
    pub fn photo_url(&self, purchase_id: &str) -> String {
        format!("{}/purchases/{purchase_id}/photo", self.api.base_path())
    }

    /// 店名から住所の候補を引く (FR-22)。
    ///
    /// 候補は Google Places API (New) の Text Search が返す場所の名前と住所で、最大 5 件である。
    /// 候補に無い住所も入力できる。
    pub async fn place_search(
        &self,
        query: &str,
        lang: &str,
    ) -> Result<Vec<PlaceCandidate>, RecordError> {
        let json = self
            .api
            .get_json(&format!(
                "/place-search?q={}&lang={}",
                encode_query(query),
                encode_query(lang)
            ))
            .await?;
        items_field(&json, "candidates", PlaceCandidate::from_json)
    }

    /// 地図の埋め込みの API キーを引く (FR-22)。キーが未設定のときは None にする (地図を出さない)。
    pub async fn maps_config(&self) -> Result<Option<String>, RecordError> {
        let json = self.api.get_json("/maps/config").await?;
        optional_string(&json, "embed_api_key")
    }
}

/// 一覧の経路に、件数と並び順と絞り込みとカーソルを付ける (FR-19、FR-20、FR-21)。
///
/// `sort` が空のときは送らない (API の既定に任せる。選択のシート)。
fn list_path(path: &str, options: ListOptions, cursor: Option<&str>, name: Option<&str>) -> String {
    let mut query = format!("?limit={PAGE_SIZE}");
    if !options.sort.is_empty() {
        query.push_str("&sort=");
        query.push_str(options.sort);
    }
    query.push_str("&order=");
    query.push_str(options.order.as_str());
    if options.favorite_only {
        query.push_str("&favorite=true");
    }
    if let Some(cursor) = cursor {
        query.push_str("&cursor=");
        query.push_str(&encode_query(cursor));
    }
    if let Some(name) = name {
        query.push_str("&name=");
        query.push_str(&encode_query(name));
    }
    format!("{path}{query}")
}

/// お気に入りを付け外しする (FR-21)。`PUT` で付け、`DELETE` で外し、更新後の記録を返す。
async fn favorite_record<T>(
    api: &ApiClient,
    resource: &str,
    id: &str,
    favorite: bool,
    parse: fn(&serde_json::Map<String, serde_json::Value>) -> Result<T, RecordError>,
) -> Result<T, RecordError> {
    let path = format!("/{resource}/{id}/favorite");
    let json = if favorite {
        api.put_json(&path).await?
    } else {
        api.delete_json(&path).await?
    };
    parse(&json)
}

/// クエリ文字列の値をパーセントエンコードする (RFC 3986 の unreserved 以外を `%XX` にする)。
///
/// 空白は `%20` にする (API のクエリの解釈はどちらも空白にする)。
pub fn encode_query(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(char::from(*byte));
            }
            _ => {
                encoded.push('%');
                encoded.push_str(&format!("{byte:02X}"));
            }
        }
    }
    encoded
}

impl From<ApiCallError> for RecordError {
    fn from(error: ApiCallError) -> Self {
        Self::Api(error)
    }
}
