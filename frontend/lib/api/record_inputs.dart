/// 記録の登録と更新の入力 (FR-6 から FR-13)。
///
/// 画面が組み立てた値を API の本文の形に直す。更新では画面が持っている全ての項目を送り、
/// 空にした任意の項目は `null` として送る (API は項目が無いときは変更しない。FR-9)。
library;

/// 店の登録と更新の入力 (FR-6)。
class ShopInput {
  const ShopInput({required this.name, this.address});

  /// 店名。必須。
  final String name;

  /// 住所。空にしたときは null。
  final String? address;

  /// API の本文にする。
  Map<String, Object?> toJson() => <String, Object?>{
    'name': name,
    'address': address,
  };
}

/// 商品の登録と更新の入力 (FR-7、FR-8)。
class ProductInput {
  const ProductInput({
    required this.name,
    this.producer,
    this.origin,
    this.region,
    this.process,
    this.variety,
    this.flavorNotes = const <String>[],
  });

  /// 商品名。必須。
  final String name;
  final String? producer;
  final String? origin;
  final String? region;
  final String? process;
  final String? variety;

  /// Flavor Notes のタグ名。更新ではこの配列で置き換える (FR-8)。
  final List<String> flavorNotes;

  /// API の本文にする。タグは空の配列でも送り、全て外す操作を表せるようにする (FR-8)。
  Map<String, Object?> toJson() => <String, Object?>{
    'name': name,
    'producer': producer,
    'origin': origin,
    'region': region,
    'process': process,
    'variety': variety,
    'flavor_notes': flavorNotes,
  };
}

/// 購入の登録と更新の入力 (FR-9)。
class PurchaseInput {
  const PurchaseInput({
    required this.productId,
    this.shopId,
    required this.purchasedOn,
    this.roast,
    this.roastDate,
    this.priceAmount,
    this.priceCurrency,
    this.weightGrams,
  });

  /// 商品の ID。必須。
  final String productId;

  /// 店の ID。店を指定しないときは null。
  final String? shopId;

  /// 購入日 (`YYYY-MM-DD`)。必須。
  final String purchasedOn;
  final String? roast;
  final String? roastDate;

  /// 価格 (通貨の最小単位)。無いときは null。
  final int? priceAmount;

  /// ISO 4217 の通貨コード。価格が無いときは null にする (API は組で扱う。FR-9)。
  final String? priceCurrency;
  final int? weightGrams;

  /// API の本文にする。
  ///
  /// 価格が無いときは通貨コードも null にする (価格だけを持つ組は API が 400 で拒否する)。
  Map<String, Object?> toJson() => <String, Object?>{
    'product_id': productId,
    'shop_id': shopId,
    'purchased_on': purchasedOn,
    'roast': roast,
    'roast_date': roastDate,
    'price_amount': priceAmount,
    'price_currency': priceAmount == null ? null : priceCurrency,
    'weight_grams': weightGrams,
  };
}

/// 抽出の登録と更新の入力 (FR-11)。
class BrewInput {
  const BrewInput({
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
  });

  /// 購入の ID。必須。
  final String purchaseId;

  /// 抽出日時 (ISO 8601 の UTC)。必須。
  final String brewedAt;
  final double? doseGrams;
  final double? waterGrams;
  final double? waterTempC;
  final int? brewTimeSeconds;
  final String? method;
  final String? grindSetting;

  /// 評価 (1 から 5)。無いときは null。
  final int? rating;
  final String? notes;

  /// API の本文にする。
  Map<String, Object?> toJson() => <String, Object?>{
    'purchase_id': purchaseId,
    'brewed_at': brewedAt,
    'dose_grams': doseGrams,
    'water_grams': waterGrams,
    'water_temp_c': waterTempC,
    'brew_time_seconds': brewTimeSeconds,
    'method': method,
    'grind_setting': grindSetting,
    'rating': rating,
    'notes': notes,
  };
}
