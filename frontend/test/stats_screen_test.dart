/// 統計の画面 (FR-18) のウィジェットテスト。
///
/// 端末の時計と UTC オフセットは固定し、期間の切り替えが API のクエリに反映されることと、
/// グラフがそれぞれの系列を持つことを確認する。
library;

import 'package:brew_book/records/clock.dart';
import 'package:brew_book/router/app_router.dart';
import 'package:brew_book/screens/home_screen.dart';
import 'package:brew_book/screens/stats_screen.dart';
import 'package:fl_chart/fl_chart.dart';
import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/fake_api.dart';
import 'support/fake_clock.dart';
import 'support/fake_passkey_client.dart';
import 'support/fake_records.dart';
import 'support/pump_app.dart';

void main() {
  /// 統計のテストが使う端末 (2026-09-25、日本標準時の +540)。
  FixedDeviceClock testClock() {
    return FixedDeviceClock(value: DateTime(2026, 9, 25, 12, 30), offsetMinutes: 540);
  }

  /// 統計の画面を開いた状態にする。
  Future<FakeApi> openStats(
    WidgetTester tester, {
    DeviceClock? clock,
    List<Map<String, Object?>> brews = const <Map<String, Object?>>[],
    List<Map<String, Object?>> purchases = const <Map<String, Object?>>[],
    List<Map<String, Object?>> ratings = const <Map<String, Object?>>[],
  }) async {
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      );
    onStats(api, brews: brews, purchases: purchases, brewRatings: ratings);
    await pumpApp(
      tester,
      apiClient: api.client(),
      passkeyClient: FakePasskeyClient(),
      clock: clock ?? testClock(),
    );
    await openLocation(tester, AppRoutes.stats);
    expect(find.byType(StatsScreen), findsOneWidget);
    return api;
  }

  /// 統計の 1 つのグラフの系列を読む。
  List<List<double>> barValues(WidgetTester tester, String key) {
    final chart = tester.widget<BarChart>(find.byKey(Key(key)));
    return chart.data.barGroups
        .map((group) => <double>[for (final rod in group.barRods) rod.toY])
        .toList();
  }

  /// 散布図の点 (x と y の組) を読む。
  List<List<double>> scatterValues(WidgetTester tester, String key) {
    final chart = tester.widget<ScatterChart>(find.byKey(Key(key)));
    return chart.data.scatterSpots.map((spot) => <double>[spot.x, spot.y]).toList();
  }

  /// 日付の入力欄に文字を入れる。
  Future<void> enterDay(WidgetTester tester, String fieldKey, String day) async {
    await tester.enterText(
      find.descendant(of: find.byKey(Key(fieldKey)), matching: find.byType(TextField)),
      day,
    );
    await tester.pumpAndSettle();
  }

  /// 期間の切り替えの選択肢を押す。横に長いため、画面内に入れてから押す。
  Future<void> tapPreset(WidgetTester tester, String preset) async {
    final chip = find.byKey(Key('stats-period-$preset'));
    await tester.ensureVisible(chip);
    await tester.pumpAndSettle();
    await tester.tap(chip);
    await tester.pumpAndSettle();
  }

  testWidgets('初期状態では当月の日別の棒グラフを表示する', (tester) async {
    final l10n = await loadL10n();
    final api = await openStats(
      tester,
      brews: <Map<String, Object?>>[
        brewPeriodJson(period: '2026-09-01', brewCount: 3, doseGrams: 45.0),
      ],
      purchases: <Map<String, Object?>>[
        purchasePeriodJson(period: '2026-09-01', priceCurrency: 'JPY'),
      ],
      ratings: <Map<String, Object?>>[
        brewRatingJson(doseGrams: 15.0, waterGrams: 250.0, waterTempC: 92.0, brewTimeSeconds: 150),
      ],
    );

    // 当月 1 日から当日までを日別で呼ぶ (FR-18)。
    expect(api.lastQuery('GET', '/api/stats/brews'), <String, String>{
      'granularity': 'day',
      'start': '2026-09-01',
      'end': '2026-09-25',
      'utc_offset_minutes': '540',
    });
    // 当月の切り替えが選ばれている。
    expect(
      tester.widget<ChoiceChip>(find.byKey(const Key('stats-period-currentMonth'))).selected,
      isTrue,
    );
    // 初期状態の棒グラフが当月の区間を持つ。
    expect(find.text(l10n.statsPeriodCurrentMonth), findsOneWidget);
    expect(barValues(tester, 'stats-brew-count-chart'), <List<double>>[
      <double>[3.0],
    ]);
    expect(find.byKey(const Key('stats-brew-dose-chart')), findsOneWidget);
  });

  testWidgets('期間の切り替えが重なっても古い読み込みの結果で上書きしない', (tester) async {
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      )
      ..onQuery('GET', '/api/stats/brews', (query) {
        // 当月は 1 件、3 か月は 2 件にして、どちらの応答が表示されたか分かるようにする。
        final count = query['start'] == '2026-09-01' ? 1 : 2;
        return (
          status: 200,
          body: <String, Object?>{
            'brews': <Map<String, Object?>>[
              brewPeriodJson(period: '2026-09-01', brewCount: count, doseGrams: count * 10.0),
            ],
          },
        );
      })
      ..onQuery(
        'GET',
        '/api/stats/purchases',
        (query) => (status: 200, body: <String, Object?>{'purchases': <Object?>[]}),
      )
      ..onQuery(
        'GET',
        '/api/stats/brew-ratings',
        (query) => (status: 200, body: <String, Object?>{'brew_ratings': <Object?>[]}),
      );
    // 最初の読み込み (当月) の最初の応答を保留し、その間に 3 か月へ切り替える
    // (2 回目の読み込みは保留せず、そのまま完了させる)。
    api.holdOnce('GET', '/api/stats/brews');
    await pumpApp(
      tester,
      apiClient: api.client(),
      passkeyClient: FakePasskeyClient(),
      clock: testClock(),
    );
    // 保留中の読み込みではスピナーが回り続けるため、画面を開くのに pumpAndSettle は使わない。
    GoRouter.of(tester.element(find.byType(HomeScreen))).go(AppRoutes.stats);
    await tester.pump();
    await tester.pump();

    // 切り替えの読み込み (3 か月) は保留されずに完了する。保留中の読み込みのスピナーが
    // 回っているため、pumpAndSettle ではなく決めた回数だけ進める。
    final chip = find.byKey(const Key('stats-period-threeMonths'));
    await tester.ensureVisible(chip);
    await tester.pump();
    await tester.tap(chip);
    for (var i = 0; i < 10; i++) {
      await tester.pump(const Duration(milliseconds: 20));
    }
    expect(barValues(tester, 'stats-brew-count-chart'), <List<double>>[
      <double>[2.0],
    ]);

    // 古い読み込み (当月) の応答を解放しても、表示と選択は 3 か月のまま。
    api.release('GET', '/api/stats/brews');
    await tester.pumpAndSettle();

    expect(barValues(tester, 'stats-brew-count-chart'), <List<double>>[
      <double>[2.0],
    ]);
    expect(
      tester.widget<ChoiceChip>(find.byKey(const Key('stats-period-threeMonths'))).selected,
      isTrue,
    );
  });

  testWidgets('集計が空の区画には記録が無いことを表示する', (tester) async {
    final l10n = await loadL10n();
    await openStats(tester);

    // 抽出の 2 つ、購入の 2 つ、散布図 4 つの区画に表示する (Charts のガイドライン)。
    expect(find.text(l10n.noRecords), findsNWidgets(8));
  });

  testWidgets('期間を 3 か月、6 か月、12 か月、全期間に切り替えられる', (tester) async {
    final api = await openStats(tester);

    Future<void> switchTo(String preset, String granularity, String? start) async {
      await tapPreset(tester, preset);
      final query = api.lastQuery('GET', '/api/stats/brews');
      expect(query?['granularity'], granularity, reason: preset);
      expect(query?['start'], start, reason: preset);
      expect(query?['end'], '2026-09-25', reason: preset);
      expect(query?['utc_offset_minutes'], '540', reason: preset);
    }

    // 3 か月、6 か月、12 か月は当月を含む直近の月数の月別にする (FR-18)。
    await switchTo('threeMonths', 'month', '2026-07-01');
    await switchTo('sixMonths', 'month', '2026-04-01');
    await switchTo('twelveMonths', 'month', '2025-10-01');

    // 全期間は開始日と終了日を省略して呼ぶ (FR-18)。
    await tapPreset(tester, 'allTime');
    expect(api.lastQuery('GET', '/api/stats/brews'), <String, String>{
      'granularity': 'month',
      'utc_offset_minutes': '540',
    });
    expect(api.lastQuery('GET', '/api/stats/purchases'), <String, String>{
      'granularity': 'month',
    });

    // 当月に戻せる。
    await switchTo('currentMonth', 'day', '2026-09-01');
  });

  testWidgets('任意の期間を指定でき、62 日以下は日別、それ以上は月別にする', (tester) async {
    final l10n = await loadL10n();
    final api = await openStats(tester);

    await tapPreset(tester, 'custom');

    // 既定値は当月の 1 日から当日まで。
    expect(find.widgetWithText(TextField, '2026-09-01'), findsOneWidget);
    expect(find.widgetWithText(TextField, '2026-09-25'), findsOneWidget);

    // 62 日以下 (2026-07-01 から 2026-08-31 までは 62 日) は日別にする。
    await enterDay(tester, 'stats-start-day', '2026-07-01');
    await enterDay(tester, 'stats-end-day', '2026-08-31');
    await tester.tap(find.widgetWithText(OutlinedButton, l10n.statsApplyButton));
    await tester.pumpAndSettle();
    expect(api.lastQuery('GET', '/api/stats/brews'), <String, String>{
      'granularity': 'day',
      'start': '2026-07-01',
      'end': '2026-08-31',
      'utc_offset_minutes': '540',
    });

    // 63 日以上は月別にする。
    await enterDay(tester, 'stats-start-day', '2026-07-01');
    await enterDay(tester, 'stats-end-day', '2026-09-01');
    await tester.tap(find.widgetWithText(OutlinedButton, l10n.statsApplyButton));
    await tester.pumpAndSettle();
    expect(api.lastQuery('GET', '/api/stats/brews')?['granularity'], 'month');
    expect(api.lastQuery('GET', '/api/stats/brews')?['start'], '2026-07-01');
    expect(api.lastQuery('GET', '/api/stats/brews')?['end'], '2026-09-01');
  });

  testWidgets('任意の期間の入力の誤りを表示し、API を呼ばない', (tester) async {
    final l10n = await loadL10n();
    final api = await openStats(tester);
    final callsBefore = api.calls.length;

    await tapPreset(tester, 'custom');

    // 形式の違反。
    await enterDay(tester, 'stats-start-day', '2026-9-1');
    await tester.tap(find.widgetWithText(OutlinedButton, l10n.statsApplyButton));
    await tester.pumpAndSettle();
    expect(find.text(l10n.validationDay), findsOneWidget);

    // 開始日が終了日より後。
    await enterDay(tester, 'stats-start-day', '2026-09-10');
    await enterDay(tester, 'stats-end-day', '2026-09-01');
    await tester.tap(find.widgetWithText(OutlinedButton, l10n.statsApplyButton));
    await tester.pumpAndSettle();
    expect(find.text(l10n.validationPeriod), findsOneWidget);
    expect(api.calls.length, callsBefore);
  });

  testWidgets('抽出回数と豆の消費量の棒グラフがそれぞれの系列を持つ', (tester) async {
    await openStats(
      tester,
      brews: <Map<String, Object?>>[
        brewPeriodJson(period: '2026-09-01', brewCount: 3, doseGrams: 45.0),
        brewPeriodJson(period: '2026-09-02', brewCount: 2, doseGrams: 30.5),
      ],
    );

    expect(barValues(tester, 'stats-brew-count-chart'), <List<double>>[
      <double>[3.0],
      <double>[2.0],
    ]);
    expect(barValues(tester, 'stats-brew-dose-chart'), <List<double>>[
      <double>[45.0],
      <double>[30.5],
    ]);
  });

  testWidgets('購入金額と重量の棒グラフが通貨コードごとの系列を持つ', (tester) async {
    await openStats(
      tester,
      purchases: <Map<String, Object?>>[
        purchasePeriodJson(period: '2026-09-01', priceCurrency: 'JPY', priceAmount: 1200, weightGrams: 200),
        purchasePeriodJson(period: '2026-09-02', priceCurrency: 'JPY', priceAmount: 800, weightGrams: 150),
        purchasePeriodJson(period: '2026-09-01', priceCurrency: 'USD', priceAmount: 1500, weightGrams: 340),
        // 価格が無い購入は通貨コードが null の組になる (FR-18)。
        purchasePeriodJson(period: '2026-09-03', priceCurrency: null, priceAmount: 0, weightGrams: 100),
      ],
    );

    // 為替換算はせず、通貨コードごとに分ける (FR-18)。
    expect(barValues(tester, 'stats-purchase-amount-chart-JPY'), <List<double>>[
      <double>[1200.0],
      <double>[800.0],
    ]);
    expect(barValues(tester, 'stats-purchase-weight-chart-JPY'), <List<double>>[
      <double>[200.0],
      <double>[150.0],
    ]);
    expect(barValues(tester, 'stats-purchase-amount-chart-USD'), <List<double>>[
      <double>[1500.0],
    ]);
    expect(barValues(tester, 'stats-purchase-weight-chart-USD'), <List<double>>[
      <double>[340.0],
    ]);
    expect(barValues(tester, 'stats-purchase-amount-chart-none'), <List<double>>[
      <double>[0.0],
    ]);
    expect(barValues(tester, 'stats-purchase-weight-chart-none'), <List<double>>[
      <double>[100.0],
    ]);
  });

  testWidgets('抽出条件と評価の関係の散布図 4 つが条件ごとの系列を持つ', (tester) async {
    await openStats(
      tester,
      ratings: <Map<String, Object?>>[
        brewRatingJson(
          id: 'brew-1',
          doseGrams: 15.0,
          waterGrams: 250.0,
          waterTempC: 92.0,
          brewTimeSeconds: 150,
          rating: 4,
        ),
        // 条件が null の項目はその散布図に描かない (FR-18)。4 つの散布図それぞれで確かめる。
        brewRatingJson(
          id: 'brew-2',
          doseGrams: null,
          waterGrams: null,
          waterTempC: null,
          brewTimeSeconds: null,
          rating: 5,
        ),
        brewRatingJson(
          id: 'brew-3',
          doseGrams: 18.0,
          waterGrams: 240.0,
          waterTempC: 88.0,
          brewTimeSeconds: 180,
          rating: 5,
        ),
      ],
    );

    expect(scatterValues(tester, 'stats-rating-dose-chart'), <List<double>>[
      <double>[15.0, 4.0],
      <double>[18.0, 5.0],
    ]);
    expect(scatterValues(tester, 'stats-rating-water-chart'), <List<double>>[
      <double>[250.0, 4.0],
      <double>[240.0, 5.0],
    ]);
    expect(scatterValues(tester, 'stats-rating-temp-chart'), <List<double>>[
      <double>[92.0, 4.0],
      <double>[88.0, 5.0],
    ]);
    expect(scatterValues(tester, 'stats-rating-time-chart'), <List<double>>[
      <double>[150.0, 4.0],
      <double>[180.0, 5.0],
    ]);
  });

  testWidgets('端末の UTC オフセット (+540) を API に渡す', (tester) async {
    final api = await openStats(
      tester,
      clock: FixedDeviceClock(value: DateTime(2026, 9, 25, 12), offsetMinutes: 540),
    );

    // 抽出の API は端末のタイムゾーンで日と月を区切るため、オフセットを渡す (FR-18)。
    expect(api.lastQuery('GET', '/api/stats/brews')?['utc_offset_minutes'], '540');
    expect(api.lastQuery('GET', '/api/stats/brew-ratings')?['utc_offset_minutes'], '540');
    // 購入日はタイムゾーンを持たないため渡さない (FR-18)。
    expect(api.lastQuery('GET', '/api/stats/purchases'), isNot(contains('utc_offset_minutes')));
  });

  testWidgets('ホームのメニューから統計画面を開ける', (tester) async {
    final l10n = await loadL10n();
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      );
    onStats(api);
    await pumpApp(
      tester,
      apiClient: api.client(),
      passkeyClient: FakePasskeyClient(),
      clock: testClock(),
    );

    await tester.tap(find.byTooltip(l10n.menuTooltip));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.statsTitle));
    await tester.pumpAndSettle();

    expect(find.byType(StatsScreen), findsOneWidget);
  });

  testWidgets('読み込みに失敗すると再試行を促し、再試行で回復する', (tester) async {
    final l10n = await loadL10n();
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      );
    onStats(api);
    api.onNetworkError('GET', '/api/stats/brews');
    await pumpApp(
      tester,
      apiClient: api.client(),
      passkeyClient: FakePasskeyClient(),
      clock: testClock(),
    );
    await openLocation(tester, AppRoutes.stats);

    expect(find.text(l10n.errorNetwork), findsOneWidget);

    // 再試行で読み込むと、失敗の表示が消えてグラフが出る。
    api.on(
      'GET',
      '/api/stats/brews',
      status: 200,
      body: <String, Object?>{
        'brews': <Object?>[
          brewPeriodJson(period: '2026-09-01', brewCount: 1, doseGrams: 15.0),
        ],
      },
    );
    await tester.tap(find.widgetWithText(TextButton, l10n.retryButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.errorNetwork), findsNothing);
    expect(barValues(tester, 'stats-brew-count-chart'), <List<double>>[
      <double>[1.0],
    ]);
  });
}
