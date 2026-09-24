/// 記録の API の応答の型 (FR-6 から FR-13)。
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

/// 一覧の 1 ページ (カーソル方式。ADR-0002)。
class RecordPage<T> {
  const RecordPage({required this.items, this.nextCursor});

  /// このページの行。
  final List<T> items;

  /// 続きを引くカーソル。続きが無ければ null。
  final String? nextCursor;
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
