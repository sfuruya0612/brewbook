/// 統合テストが使う記録の JSON (API の応答の形。FR-9、FR-11)。
///
/// `flutter drive` の Web のビルドは `test/` の補助を読めないため、ここに置く。
library;

/// 商品の応答。
Map<String, Object?> productJson({String id = 'product-1', String name = 'Test Product'}) {
  return <String, Object?>{
    'id': id,
    'user_id': 'user-1',
    'name': name,
    'producer': null,
    'origin': null,
    'region': null,
    'process': null,
    'variety': null,
    'flavor_notes': <String>[],
    'created_at': '2026-09-01T00:00:00.000Z',
    'updated_at': '2026-09-01T00:00:00.000Z',
    'archived_at': null,
  };
}

/// 購入の応答 (商品をネストする。FR-9)。
Map<String, Object?> purchaseJson({String id = 'purchase-1', Map<String, Object?>? product}) {
  return <String, Object?>{
    'id': id,
    'user_id': 'user-1',
    'product_id': (product ?? productJson())['id'],
    'shop_id': null,
    'purchased_on': '2026-09-01',
    'roast': null,
    'roast_date': null,
    'price_amount': null,
    'price_currency': null,
    'weight_grams': null,
    'photo_key': null,
    'created_at': '2026-09-01T00:00:00.000Z',
    'updated_at': '2026-09-01T00:00:00.000Z',
    'archived_at': null,
    'product': product ?? productJson(),
    'shop': null,
  };
}

/// 抽出の応答 (購入をネストする。FR-11)。
Map<String, Object?> brewJson({
  String id = 'brew-1',
  String brewedAt = '2026-09-01T00:00:00.000Z',
  Map<String, Object?>? purchase,
}) {
  return <String, Object?>{
    'id': id,
    'user_id': 'user-1',
    'purchase_id': (purchase ?? purchaseJson())['id'],
    'brewed_at': brewedAt,
    'dose_grams': null,
    'water_grams': null,
    'water_temp_c': null,
    'brew_time_seconds': null,
    'method': null,
    'grind_setting': null,
    'rating': null,
    'notes': null,
    'created_at': '2026-09-01T00:00:00.000Z',
    'updated_at': '2026-09-01T00:00:00.000Z',
    'archived_at': null,
    'purchase': purchase ?? purchaseJson(),
  };
}

/// カーソル方式の一覧の応答 (ADR-0002)。
Map<String, Object?> pageJson({
  required String key,
  required List<Map<String, Object?>> items,
  String? nextCursor,
}) {
  return <String, Object?>{key: items, 'next_cursor': nextCursor};
}
