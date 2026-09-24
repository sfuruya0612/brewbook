/// テスト用の記録の JSON (API の応答の形。FR-6 から FR-13)。
///
/// 画面はこの形だけを読むため、テストは必要な項目だけを変えて応答を組み立てる。
library;

import 'fake_api.dart';

/// 店の応答。
Map<String, Object?> shopJson({
  String id = 'shop-1',
  String name = 'Test Shop',
  String? address,
  String? archivedAt,
}) {
  return <String, Object?>{
    'id': id,
    'user_id': 'user-1',
    'name': name,
    'address': address,
    'created_at': '2026-09-01T00:00:00.000Z',
    'updated_at': '2026-09-01T00:00:00.000Z',
    'archived_at': archivedAt,
  };
}

/// 商品の応答 (タグを含む。FR-8)。
Map<String, Object?> productJson({
  String id = 'product-1',
  String name = 'Test Product',
  String? producer,
  String? origin,
  String? region,
  String? process,
  String? variety,
  List<String> flavorNotes = const <String>[],
  String? archivedAt,
}) {
  return <String, Object?>{
    'id': id,
    'user_id': 'user-1',
    'name': name,
    'producer': producer,
    'origin': origin,
    'region': region,
    'process': process,
    'variety': variety,
    'flavor_notes': flavorNotes,
    'created_at': '2026-09-01T00:00:00.000Z',
    'updated_at': '2026-09-01T00:00:00.000Z',
    'archived_at': archivedAt,
  };
}

/// 購入の応答 (商品と店をネストする。FR-9)。
Map<String, Object?> purchaseJson({
  String id = 'purchase-1',
  String purchasedOn = '2026-09-01',
  String? roast,
  String? roastDate,
  int? priceAmount,
  String? priceCurrency,
  int? weightGrams,
  String? photoKey,
  String? archivedAt,
  Map<String, Object?>? product,
  Map<String, Object?>? shop,
}) {
  return <String, Object?>{
    'id': id,
    'user_id': 'user-1',
    'product_id': (product ?? productJson())['id'],
    'shop_id': shop == null ? null : shop['id'],
    'purchased_on': purchasedOn,
    'roast': roast,
    'roast_date': roastDate,
    'price_amount': priceAmount,
    'price_currency': priceCurrency,
    'weight_grams': weightGrams,
    'photo_key': photoKey,
    'created_at': '2026-09-01T00:00:00.000Z',
    'updated_at': '2026-09-01T00:00:00.000Z',
    'archived_at': archivedAt,
    'product': product ?? productJson(),
    'shop': shop,
  };
}

/// 抽出の応答 (購入をネストする。FR-11)。
Map<String, Object?> brewJson({
  String id = 'brew-1',
  String brewedAt = '2026-09-01T00:00:00.000Z',
  double? doseGrams,
  double? waterGrams,
  double? waterTempC,
  int? brewTimeSeconds,
  String? method,
  String? grindSetting,
  int? rating,
  String? notes,
  String? archivedAt,
  Map<String, Object?>? purchase,
}) {
  return <String, Object?>{
    'id': id,
    'user_id': 'user-1',
    'purchase_id': (purchase ?? purchaseJson())['id'],
    'brewed_at': brewedAt,
    'dose_grams': doseGrams,
    'water_grams': waterGrams,
    'water_temp_c': waterTempC,
    'brew_time_seconds': brewTimeSeconds,
    'method': method,
    'grind_setting': grindSetting,
    'rating': rating,
    'notes': notes,
    'created_at': '2026-09-01T00:00:00.000Z',
    'updated_at': '2026-09-01T00:00:00.000Z',
    'archived_at': archivedAt,
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

/// サジェストの応答を登録する (入力中の文字列で前方一致する候補だけを返す。FR-13)。
void onSuggestion(FakeApi api, String field, String candidate) {
  api.onQuery('GET', '/api/suggestions/$field', (query) {
    final q = (query['q'] ?? '').toLowerCase();
    return (
      status: 200,
      body: <String, Object?>{
        'values': candidate.toLowerCase().startsWith(q) ? <String>[candidate] : <String>[],
      },
    );
  });
}
