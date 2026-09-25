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

/// 抽出回数と豆の消費量の 1 区間の応答 (FR-18)。
Map<String, Object?> brewPeriodJson({
  String period = '2026-09-01',
  int brewCount = 1,
  double doseGrams = 15.0,
}) {
  return <String, Object?>{
    'period': period,
    'brew_count': brewCount,
    'dose_grams': doseGrams,
  };
}

/// 購入金額と重量の 1 区間と通貨コードの組の応答 (FR-18)。
Map<String, Object?> purchasePeriodJson({
  String period = '2026-09-01',
  String? priceCurrency = 'JPY',
  int priceAmount = 1200,
  int weightGrams = 200,
  int purchaseCount = 1,
}) {
  return <String, Object?>{
    'period': period,
    'price_currency': priceCurrency,
    'price_amount': priceAmount,
    'weight_grams': weightGrams,
    'purchase_count': purchaseCount,
  };
}

/// 抽出条件と評価の関係の 1 件の応答 (FR-18)。
Map<String, Object?> brewRatingJson({
  String id = 'brew-1',
  double? doseGrams,
  double? waterGrams,
  double? waterTempC,
  int? brewTimeSeconds,
  int rating = 4,
}) {
  return <String, Object?>{
    'id': id,
    'dose_grams': doseGrams,
    'water_grams': waterGrams,
    'water_temp_c': waterTempC,
    'brew_time_seconds': brewTimeSeconds,
    'rating': rating,
  };
}

/// 購入ごとの評価の推移の 1 件の応答 (FR-18)。
Map<String, Object?> ratingHistoryJson({
  String id = 'brew-1',
  String brewedAt = '2026-09-01T00:00:00.000Z',
  int rating = 4,
}) {
  return <String, Object?>{
    'id': id,
    'brewed_at': brewedAt,
    'rating': rating,
  };
}

/// 統計の API の応答を登録する (FR-18)。
///
/// 統計画面は 3 つの API を呼ぶため、テストは 1 回の登録で全ての応答を用意できる。
void onStats(
  FakeApi api, {
  List<Map<String, Object?>> brews = const <Map<String, Object?>>[],
  List<Map<String, Object?>> purchases = const <Map<String, Object?>>[],
  List<Map<String, Object?>> brewRatings = const <Map<String, Object?>>[],
}) {
  api
    ..on('GET', '/api/stats/brews', status: 200, body: <String, Object?>{'brews': brews})
    ..on(
      'GET',
      '/api/stats/purchases',
      status: 200,
      body: <String, Object?>{'purchases': purchases},
    )
    ..on(
      'GET',
      '/api/stats/brew-ratings',
      status: 200,
      body: <String, Object?>{'brew_ratings': brewRatings},
    );
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
