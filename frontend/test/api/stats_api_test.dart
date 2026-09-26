/// `StatsApi` の呼び出しの単体テスト (FR-18)。
///
/// 期間と粒度と UTC オフセットがクエリパラメータに渡ることと、応答の読み取りを確認する。
library;

import 'package:brew_book/api/stats_api.dart';
import 'package:flutter_test/flutter_test.dart';

import '../support/fake_api.dart';

void main() {
  test('抽出の統計に期間と粒度と UTC オフセットを渡す', () async {
    final api = FakeApi()..on('GET', '/api/stats/brews', status: 200, body: <String, Object?>{
      'brews': <Object?>[],
    });
    final stats = StatsApi(api.client());

    await stats.brews(
      start: '2026-09-01',
      end: '2026-09-25',
      granularity: StatsGranularity.day,
      utcOffsetMinutes: 540,
    );

    expect(api.lastQuery('GET', '/api/stats/brews'), <String, String>{
      'granularity': 'day',
      'start': '2026-09-01',
      'end': '2026-09-25',
      'utc_offset_minutes': '540',
    });
  });

  test('購入の統計は期間と粒度だけを渡し、UTC オフセットを渡さない', () async {
    final api = FakeApi()
      ..on('GET', '/api/stats/purchases', status: 200, body: <String, Object?>{
        'purchases': <Object?>[],
      });
    final stats = StatsApi(api.client());

    await stats.purchases(granularity: StatsGranularity.month);

    expect(api.lastQuery('GET', '/api/stats/purchases'), <String, String>{
      'granularity': 'month',
    });
  });

  test('応答の区間と条件を読む (条件が null の場合を含む)', () async {
    final api = FakeApi()
      ..on('GET', '/api/stats/brews', status: 200, body: <String, Object?>{
        'brews': <Object?>[
          <String, Object?>{'period': '2026-09-01', 'brew_count': 3, 'dose_grams': 45.5},
        ],
      })
      ..on('GET', '/api/stats/purchases', status: 200, body: <String, Object?>{
        'purchases': <Object?>[
          <String, Object?>{
            'period': '2026-09',
            'price_currency': null,
            'price_amount': 0,
            'weight_grams': 200,
            'purchase_count': 1,
          },
        ],
      })
      ..on('GET', '/api/stats/brew-ratings', status: 200, body: <String, Object?>{
        'brew_ratings': <Object?>[
          <String, Object?>{
            'id': 'brew-1',
            'dose_grams': 15.0,
            'water_grams': null,
            'water_temp_c': 92.0,
            'brew_time_seconds': null,
            'rating': 5,
          },
        ],
      });
    final stats = StatsApi(api.client());

    final brews = await stats.brews(
      granularity: StatsGranularity.day,
      utcOffsetMinutes: 540,
    );
    expect(brews.single.period, '2026-09-01');
    expect(brews.single.brewCount, 3);
    expect(brews.single.doseGrams, 45.5);

    final purchases = await stats.purchases(granularity: StatsGranularity.month);
    expect(purchases.single.period, '2026-09');
    expect(purchases.single.priceCurrency, isNull);
    expect(purchases.single.priceAmount, 0);
    expect(purchases.single.weightGrams, 200);
    expect(purchases.single.purchaseCount, 1);

    final ratings = await stats.brewRatings(utcOffsetMinutes: 540);
    expect(ratings.single.id, 'brew-1');
    expect(ratings.single.doseGrams, 15.0);
    expect(ratings.single.waterGrams, isNull);
    expect(ratings.single.waterTempC, 92.0);
    expect(ratings.single.brewTimeSeconds, isNull);
    expect(ratings.single.rating, 5);
  });

  test('購入ごとの評価の推移は購入の経路を呼ぶ', () async {
    final api = FakeApi()
      ..on(
        'GET',
        '/api/purchases/purchase-1/rating-history',
        status: 200,
        body: <String, Object?>{
          'ratings': <Object?>[
            <String, Object?>{
              'id': 'brew-1',
              'brewed_at': '2026-09-01T00:00:00.000Z',
              'rating': 4,
            },
          ],
        },
      );
    final stats = StatsApi(api.client());

    final ratings = await stats.ratingHistory('purchase-1');

    expect(api.calls, contains('GET /api/purchases/purchase-1/rating-history'));
    expect(ratings.single.brewedAt, '2026-09-01T00:00:00.000Z');
    expect(ratings.single.rating, 4);
  });

  test('応答の区間のキーが文字列でなければ形式の違反にする', () async {
    final api = FakeApi()
      ..on('GET', '/api/stats/brews', status: 200, body: <String, Object?>{
        'brews': <Object?>[
          <String, Object?>{'period': 20260901, 'brew_count': 1, 'dose_grams': 15.0},
        ],
      });
    final stats = StatsApi(api.client());

    await expectLater(
      stats.brews(granularity: StatsGranularity.day, utcOffsetMinutes: 540),
      throwsFormatException,
    );
  });

  test('抽出条件と評価の関係に期間と UTC オフセットを渡し、粒度は渡さない', () async {
    final api = FakeApi()
      ..on('GET', '/api/stats/brew-ratings', status: 200, body: <String, Object?>{
        'brew_ratings': <Object?>[],
      });
    final stats = StatsApi(api.client());

    await stats.brewRatings(start: '2026-09-01', end: '2026-09-25', utcOffsetMinutes: -300);

    expect(api.lastQuery('GET', '/api/stats/brew-ratings'), <String, String>{
      'start': '2026-09-01',
      'end': '2026-09-25',
      'utc_offset_minutes': '-300',
    });
  });
}
