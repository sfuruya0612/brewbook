/// 記録の API の応答の型 (FR-6 から FR-13、FR-18)。
///
/// 項目名はスキーマの列名に対応させ、API の応答をそのまま表す。画面はこの型から表示を作る。
/// 日付と日時は API の形式の文字列のまま保持し、端末のタイムゾーンへの変換は表示の直前に行う。
library;

/// 店 (FR-6)。
class Shop {
  const Shop({
    required this.id,
    required this.name,
    this.address,
    required this.createdAt,
    required this.updatedAt,
    this.archivedAt,
  });

  /// JSON のオブジェクトから組み立てる。
  factory Shop.fromJson(Map<String, Object?> json) {
    return Shop(
      id: _string(json, 'id'),
      name: _string(json, 'name'),
      address: _optionalString(json, 'address'),
      createdAt: _string(json, 'created_at'),
      updatedAt: _string(json, 'updated_at'),
      archivedAt: _optionalString(json, 'archived_at'),
    );
  }

  final String id;
  final String name;
  final String? address;
  final String createdAt;
  final String updatedAt;

  /// アーカイブした日時。アーカイブ済みでなければ null (FR-12)。
  final String? archivedAt;

  /// アーカイブ済みか (FR-12)。
  bool get isArchived => archivedAt != null;
}

/// 商品 (FR-7、FR-8)。
class Product {
  const Product({
    required this.id,
    required this.name,
    this.producer,
    this.origin,
    this.region,
    this.process,
    this.variety,
    this.flavorNotes = const <String>[],
    required this.createdAt,
    required this.updatedAt,
    this.archivedAt,
  });

  /// JSON のオブジェクトから組み立てる。
  factory Product.fromJson(Map<String, Object?> json) {
    return Product(
      id: _string(json, 'id'),
      name: _string(json, 'name'),
      producer: _optionalString(json, 'producer'),
      origin: _optionalString(json, 'origin'),
      region: _optionalString(json, 'region'),
      process: _optionalString(json, 'process'),
      variety: _optionalString(json, 'variety'),
      flavorNotes: _stringList(json, 'flavor_notes'),
      createdAt: _string(json, 'created_at'),
      updatedAt: _string(json, 'updated_at'),
      archivedAt: _optionalString(json, 'archived_at'),
    );
  }

  final String id;
  final String name;
  final String? producer;
  final String? origin;
  final String? region;
  final String? process;
  final String? variety;

  /// Flavor Notes のタグ名 (名前の昇順。FR-8)。
  final List<String> flavorNotes;
  final String createdAt;
  final String updatedAt;

  /// アーカイブした日時。アーカイブ済みでなければ null (FR-12)。
  final String? archivedAt;

  /// アーカイブ済みか (FR-12)。
  bool get isArchived => archivedAt != null;
}

/// 購入 (FR-9、FR-10)。
class Purchase {
  const Purchase({
    required this.id,
    required this.productId,
    this.shopId,
    required this.purchasedOn,
    this.roast,
    this.roastDate,
    this.priceAmount,
    this.priceCurrency,
    this.weightGrams,
    this.photoKey,
    required this.createdAt,
    required this.updatedAt,
    this.archivedAt,
    required this.product,
    this.shop,
  });

  /// JSON のオブジェクトから組み立てる。
  factory Purchase.fromJson(Map<String, Object?> json) {
    final shop = _optionalObject(json, 'shop');
    return Purchase(
      id: _string(json, 'id'),
      productId: _string(json, 'product_id'),
      shopId: _optionalString(json, 'shop_id'),
      purchasedOn: _string(json, 'purchased_on'),
      roast: _optionalString(json, 'roast'),
      roastDate: _optionalString(json, 'roast_date'),
      priceAmount: _optionalInt(json, 'price_amount'),
      priceCurrency: _optionalString(json, 'price_currency'),
      weightGrams: _optionalInt(json, 'weight_grams'),
      photoKey: _optionalString(json, 'photo_key'),
      createdAt: _string(json, 'created_at'),
      updatedAt: _string(json, 'updated_at'),
      archivedAt: _optionalString(json, 'archived_at'),
      product: Product.fromJson(_object(json, 'product')),
      shop: shop == null ? null : Shop.fromJson(shop),
    );
  }

  final String id;
  final String productId;
  final String? shopId;

  /// 購入日 (`YYYY-MM-DD`)。端末のタイムゾーンでの日付 (FR-9)。
  final String purchasedOn;
  final String? roast;
  final String? roastDate;
  final int? priceAmount;
  final String? priceCurrency;
  final int? weightGrams;

  /// 写真のオブジェクトキー。写真が無ければ null (FR-10)。
  final String? photoKey;
  final String createdAt;
  final String updatedAt;

  /// アーカイブした日時。アーカイブ済みでなければ null (FR-12)。
  final String? archivedAt;

  /// 商品。必須の参照のため常にある (FR-9)。
  final Product product;

  /// 店。店が無い購入では null になる (FR-9)。
  final Shop? shop;

  /// アーカイブ済みか (FR-12)。
  bool get isArchived => archivedAt != null;
}

/// 抽出 (FR-11)。
class Brew {
  const Brew({
    required this.id,
    required this.purchaseId,
    required this.brewedAt,
    this.doseGrams,
    this.waterGrams,
    this.waterTempC,
    this.brewTimeSeconds,
    this.method,
    this.grindSetting,
    this.rating,
    this.notes,
    required this.createdAt,
    required this.updatedAt,
    this.archivedAt,
    required this.purchase,
  });

  /// JSON のオブジェクトから組み立てる。
  factory Brew.fromJson(Map<String, Object?> json) {
    return Brew(
      id: _string(json, 'id'),
      purchaseId: _string(json, 'purchase_id'),
      brewedAt: _string(json, 'brewed_at'),
      doseGrams: _optionalDouble(json, 'dose_grams'),
      waterGrams: _optionalDouble(json, 'water_grams'),
      waterTempC: _optionalDouble(json, 'water_temp_c'),
      brewTimeSeconds: _optionalInt(json, 'brew_time_seconds'),
      method: _optionalString(json, 'method'),
      grindSetting: _optionalString(json, 'grind_setting'),
      rating: _optionalInt(json, 'rating'),
      notes: _optionalString(json, 'notes'),
      createdAt: _string(json, 'created_at'),
      updatedAt: _string(json, 'updated_at'),
      archivedAt: _optionalString(json, 'archived_at'),
      purchase: Purchase.fromJson(_object(json, 'purchase')),
    );
  }

  final String id;
  final String purchaseId;

  /// 抽出日時 (ISO 8601 の UTC)。表示のときに端末のタイムゾーンへ変換する (FR-11)。
  final String brewedAt;
  final double? doseGrams;
  final double? waterGrams;
  final double? waterTempC;
  final int? brewTimeSeconds;
  final String? method;
  final String? grindSetting;

  /// 評価 (1 から 5)。無ければ null。
  final int? rating;
  final String? notes;
  final String createdAt;
  final String updatedAt;

  /// アーカイブした日時。アーカイブ済みでなければ null (FR-12)。
  final String? archivedAt;

  /// 購入。必須の参照のため常にある。中に商品と店を含む (FR-11)。
  final Purchase purchase;

  /// アーカイブ済みか (FR-12)。
  bool get isArchived => archivedAt != null;
}

/// 写真から推測した商品の項目 (FR-19)。推測できない項目は null。
///
/// 商品の応答 (`Product`) とは違い、ID と日時を持たない候補である。
class ProductSuggestion {
  const ProductSuggestion({
    this.name,
    this.producer,
    this.origin,
    this.region,
    this.process,
    this.variety,
    this.flavorNotes = const <String>[],
  });

  /// JSON のオブジェクトから組み立てる。
  factory ProductSuggestion.fromJson(Map<String, Object?> json) {
    return ProductSuggestion(
      name: _optionalString(json, 'name'),
      producer: _optionalString(json, 'producer'),
      origin: _optionalString(json, 'origin'),
      region: _optionalString(json, 'region'),
      process: _optionalString(json, 'process'),
      variety: _optionalString(json, 'variety'),
      flavorNotes: _stringList(json, 'flavor_notes'),
    );
  }

  final String? name;
  final String? producer;
  final String? origin;
  final String? region;
  final String? process;
  final String? variety;

  /// Flavor Notes の候補 (FR-8)。
  final List<String> flavorNotes;
}

/// 写真から推測した購入と商品の項目 (FR-19)。キーは購入の応答の列名に揃える。
class PurchaseSuggestion {
  const PurchaseSuggestion({
    this.product,
    this.roast,
    this.roastDate,
    this.priceAmount,
    this.weightGrams,
  });

  /// JSON のオブジェクトから組み立てる。
  factory PurchaseSuggestion.fromJson(Map<String, Object?> json) {
    final product = _optionalObject(json, 'product');
    return PurchaseSuggestion(
      product: product == null ? null : ProductSuggestion.fromJson(product),
      roast: _optionalString(json, 'roast'),
      roastDate: _optionalString(json, 'roast_date'),
      priceAmount: _optionalInt(json, 'price_amount'),
      weightGrams: _optionalInt(json, 'weight_grams'),
    );
  }

  /// 商品の項目。1 つも推測できないときは null。
  final ProductSuggestion? product;
  final String? roast;
  final String? roastDate;

  /// 価格 (整数)。通貨は推測しない (画面の既定値の JPY のまま。FR-19)。
  final int? priceAmount;
  final int? weightGrams;
}

/// 一覧の 1 ページ (カーソル方式。ADR-0002)。
class RecordPage<T> {
  const RecordPage({required this.items, this.nextCursor});

  /// このページの行。
  final List<T> items;

  /// 続きを引くカーソル。続きが無ければ null。
  final String? nextCursor;
}

/// 抽出回数と豆の消費量の 1 区間 (FR-18)。
class BrewPeriod {
  const BrewPeriod({required this.period, required this.brewCount, required this.doseGrams});

  /// JSON のオブジェクトから組み立てる。
  factory BrewPeriod.fromJson(Map<String, Object?> json) {
    return BrewPeriod(
      period: _string(json, 'period'),
      brewCount: _int(json, 'brew_count'),
      doseGrams: _double(json, 'dose_grams'),
    );
  }

  /// 区間のキー (日別は `YYYY-MM-DD`、月別は `YYYY-MM`)。
  final String period;

  /// 区間内の抽出の件数。
  final int brewCount;

  /// 区間内の豆の量の合計 (グラム)。
  final double doseGrams;
}

/// 購入金額と重量の 1 区間と通貨コードの組 (FR-18)。
class PurchasePeriod {
  const PurchasePeriod({
    required this.period,
    this.priceCurrency,
    required this.priceAmount,
    required this.weightGrams,
    required this.purchaseCount,
  });

  /// JSON のオブジェクトから組み立てる。
  factory PurchasePeriod.fromJson(Map<String, Object?> json) {
    return PurchasePeriod(
      period: _string(json, 'period'),
      priceCurrency: _optionalString(json, 'price_currency'),
      priceAmount: _int(json, 'price_amount'),
      weightGrams: _int(json, 'weight_grams'),
      purchaseCount: _int(json, 'purchase_count'),
    );
  }

  /// 区間のキー (日別は `YYYY-MM-DD`、月別は `YYYY-MM`)。
  final String period;

  /// ISO 4217 の通貨コード。価格が無い購入は null。
  final String? priceCurrency;

  /// 区間内の価格の合計 (通貨の最小単位)。
  final int priceAmount;

  /// 区間内の重量の合計 (グラム)。
  final int weightGrams;

  /// 区間内の購入の件数。
  final int purchaseCount;
}

/// 抽出条件と評価の関係の 1 件の抽出 (FR-18)。
class BrewRating {
  const BrewRating({
    required this.id,
    this.doseGrams,
    this.waterGrams,
    this.waterTempC,
    this.brewTimeSeconds,
    required this.rating,
  });

  /// JSON のオブジェクトから組み立てる。
  factory BrewRating.fromJson(Map<String, Object?> json) {
    return BrewRating(
      id: _string(json, 'id'),
      doseGrams: _optionalDouble(json, 'dose_grams'),
      waterGrams: _optionalDouble(json, 'water_grams'),
      waterTempC: _optionalDouble(json, 'water_temp_c'),
      brewTimeSeconds: _optionalInt(json, 'brew_time_seconds'),
      rating: _int(json, 'rating'),
    );
  }

  /// 抽出の ID。
  final String id;

  /// 豆の量 (グラム)。条件が無ければ null。
  final double? doseGrams;

  /// 湯量 (グラム)。条件が無ければ null。
  final double? waterGrams;

  /// 湯の温度 (摂氏)。条件が無ければ null。
  final double? waterTempC;

  /// 時間 (秒)。条件が無ければ null。
  final int? brewTimeSeconds;

  /// 評価 (1 から 5)。
  final int rating;
}

/// 購入ごとの評価の推移の 1 件の抽出 (FR-18)。
class RatingHistoryEntry {
  const RatingHistoryEntry({
    required this.id,
    required this.brewedAt,
    required this.rating,
  });

  /// JSON のオブジェクトから組み立てる。
  factory RatingHistoryEntry.fromJson(Map<String, Object?> json) {
    return RatingHistoryEntry(
      id: _string(json, 'id'),
      brewedAt: _string(json, 'brewed_at'),
      rating: _int(json, 'rating'),
    );
  }

  /// 抽出の ID。
  final String id;

  /// 抽出日時 (ISO 8601 の UTC)。
  final String brewedAt;

  /// 評価 (1 から 5)。
  final int rating;
}

/// 文字列の項目を読む。
String _string(Map<String, Object?> json, String key) {
  final value = json[key];
  if (value is String) {
    return value;
  }
  throw FormatException('the $key field must be a string but was $value');
}

/// 文字列の項目を読む。無い場合と `null` は null にする。
String? _optionalString(Map<String, Object?> json, String key) {
  final value = json[key];
  if (value == null) {
    return null;
  }
  if (value is String) {
    return value;
  }
  throw FormatException('the $key field must be a string but was $value');
}

/// 整数の項目を読む。無い場合と `null` は形式の違反にする (必須の項目)。
int _int(Map<String, Object?> json, String key) {
  final value = _optionalInt(json, key);
  if (value == null) {
    throw FormatException('the $key field must be a number but was ${json[key]}');
  }
  return value;
}

/// 小数の項目を読む。無い場合と `null` は形式の違反にする (必須の項目)。
double _double(Map<String, Object?> json, String key) {
  final value = _optionalDouble(json, key);
  if (value == null) {
    throw FormatException('the $key field must be a number but was ${json[key]}');
  }
  return value;
}

/// 整数の項目を読む。無い場合と `null` は null にする。
int? _optionalInt(Map<String, Object?> json, String key) {
  final value = json[key];
  if (value == null) {
    return null;
  }
  if (value is num) {
    return value.toInt();
  }
  throw FormatException('the $key field must be a number but was $value');
}

/// 小数の項目を読む。無い場合と `null` は null にする。
double? _optionalDouble(Map<String, Object?> json, String key) {
  final value = json[key];
  if (value == null) {
    return null;
  }
  if (value is num) {
    return value.toDouble();
  }
  throw FormatException('the $key field must be a number but was $value');
}

/// 文字列の配列の項目を読む。無い場合は空の配列にする。
List<String> _stringList(Map<String, Object?> json, String key) {
  final value = json[key];
  if (value == null) {
    return const <String>[];
  }
  if (value is List<Object?>) {
    return value.whereType<String>().toList();
  }
  throw FormatException('the $key field must be an array but was $value');
}

/// ネストしたオブジェクトの項目を読む。
Map<String, Object?> _object(Map<String, Object?> json, String key) {
  final value = json[key];
  if (value is Map<String, Object?>) {
    return value;
  }
  throw FormatException('the $key field must be an object but was $value');
}

/// ネストしたオブジェクトの項目を読む。無い場合と `null` は null にする。
Map<String, Object?>? _optionalObject(Map<String, Object?> json, String key) {
  final value = json[key];
  if (value == null) {
    return null;
  }
  if (value is Map<String, Object?>) {
    return value;
  }
  throw FormatException('the $key field must be an object but was $value');
}
