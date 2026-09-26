import 'package:brew_book/records/values.dart';
import 'package:brew_book/router/app_router.dart';
import 'package:brew_book/screens/brew_detail_screen.dart';
import 'package:brew_book/screens/brew_form_screen.dart';
import 'package:brew_book/screens/login_screen.dart';
import 'package:brew_book/screens/product_form_screen.dart';
import 'package:brew_book/screens/purchase_detail_screen.dart';
import 'package:brew_book/screens/shop_form_screen.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/fake_api.dart';
import 'support/fake_passkey_client.dart';
import 'support/fake_records.dart';
import 'support/pump_app.dart';

/// 抽出の画面 (詳細、登録、編集、アーカイブ。FR-11、FR-12) のウィジェットテスト。
void main() {
  /// 抽出の詳細の画面を開いた状態にする。
  Future<FakeApi> openBrewDetail(
    WidgetTester tester, {
    Map<String, Object?>? brew,
  }) async {
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      )
      ..on('GET', '/api/brews/brew-1', status: 200, body: brew ?? brewJson(id: 'brew-1'));
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());
    await openLocation(tester, AppRoutes.brewPath('brew-1'));
    expect(find.byType(BrewDetailScreen), findsOneWidget);
    return api;
  }

  /// 抽出の登録と編集の画面を開いた状態にする。
  Future<FakeApi> openBrewForm(
    WidgetTester tester, {
    String? id,
    Map<String, Object?>? brew,
  }) async {
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      )
      ..on(
        'GET',
        '/api/brews/brew-1',
        status: 200,
        body: brew ?? brewJson(id: 'brew-1'),
      )
      ..on(
        'GET',
        '/api/purchases',
        status: 200,
        body: pageJson(
          key: 'purchases',
          items: <Map<String, Object?>>[purchaseJson(id: 'purchase-1')],
        ),
      );
    onSuggestion(api, 'method', 'V60');
    onSuggestion(api, 'grind_setting', 'Medium Fine');
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());
    await openLocation(tester, id == null ? AppRoutes.brewNew : AppRoutes.brewEditPath(id));
    expect(find.byType(BrewFormScreen), findsOneWidget);
    return api;
  }

  testWidgets('詳細から購入と商品と店をたどれる', (tester) async {
    final l10n = await loadL10n();
    final api = await openBrewDetail(
      tester,
      brew: brewJson(
        id: 'brew-1',
        brewedAt: '2026-09-01T00:30:00.000Z',
        doseGrams: 15.0,
        waterGrams: 250.0,
        waterTempC: 92.0,
        brewTimeSeconds: 150,
        method: 'V60',
        grindSetting: 'Medium',
        rating: 4,
        notes: 'Good',
        purchase: purchaseJson(
          id: 'purchase-1',
          purchasedOn: '2026-08-01',
          product: productJson(id: 'product-1', name: 'Ethiopia'),
          shop: shopJson(id: 'shop-1', name: 'Test Shop'),
        ),
      ),
    );
    // たどる先の画面が読む API (UC-6)。
    api
      ..on('GET', '/api/purchases/purchase-1', status: 200, body: purchaseJson(id: 'purchase-1'))
      ..on('GET', '/api/products/product-1', status: 200, body: productJson(id: 'product-1'))
      ..on('GET', '/api/shops/shop-1', status: 200, body: shopJson(id: 'shop-1'));

    // 抽出日時は端末のタイムゾーンで表示する (FR-11)。
    final locale = const Locale('en');
    expect(
      find.text(displayTimestamp('2026-09-01T00:30:00.000Z', locale)),
      findsOneWidget,
    );
    expect(find.text(l10n.gramsValue('15')), findsOneWidget);
    expect(find.text(l10n.celsiusValue('92')), findsOneWidget);
    expect(find.text(l10n.secondsValue('150')), findsOneWidget);
    expect(find.text(l10n.ratingValue('4')), findsOneWidget);
    expect(find.text('V60'), findsOneWidget);

    // 購入をたどる。
    await tester.tap(find.text(displayDay('2026-08-01', locale)));
    await tester.pumpAndSettle();
    expect(find.byType(PurchaseDetailScreen), findsOneWidget);
    await tester.pageBack();
    await tester.pumpAndSettle();

    // 商品をたどる。
    await tester.tap(find.text('Ethiopia'));
    await tester.pumpAndSettle();
    expect(find.byType(ProductFormScreen), findsOneWidget);
    await tester.pageBack();
    await tester.pumpAndSettle();

    // 店をたどる。
    await tester.tap(find.text('Test Shop'));
    await tester.pumpAndSettle();
    expect(find.byType(ShopFormScreen), findsOneWidget);
  });

  testWidgets('詳細でアーカイブとアーカイブ解除ができる', (tester) async {
    final l10n = await loadL10n();
    // アーカイブの後は、アーカイブ済みの抽出を返す (サーバーの状態を模す)。
    var archived = false;
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      )
      ..onQuery('GET', '/api/brews/brew-1', (query) {
        return (
          status: 200,
          body: brewJson(
            id: 'brew-1',
            archivedAt: archived ? '2026-09-02T00:00:00.000Z' : null,
          ),
        );
      })
      ..onQuery('POST', '/api/brews/brew-1/archive', (query) {
        archived = true;
        return (status: 200, body: brewJson(id: 'brew-1', archivedAt: '2026-09-02T00:00:00.000Z'));
      })
      ..onQuery('POST', '/api/brews/brew-1/unarchive', (query) {
        archived = false;
        return (status: 200, body: brewJson(id: 'brew-1'));
      });
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());
    await openLocation(tester, AppRoutes.brewPath('brew-1'));

    await tester.tap(find.byTooltip(l10n.archiveButton));
    await tester.pumpAndSettle();

    expect(api.calls, contains('POST /api/brews/brew-1/archive'));
    expect(find.text(l10n.archivedMessage), findsOneWidget);
    // 通知が消えるまで進める (次の通知が待ち行列に残らないようにする)。
    await tester.pump(const Duration(seconds: 5));
    await tester.pumpAndSettle();

    // アーカイブ済みの抽出は、単件取得でき、アーカイブ解除で戻せる (FR-12)。
    await tester.tap(find.byTooltip(l10n.unarchiveButton));
    await tester.pumpAndSettle();

    expect(api.calls, contains('POST /api/brews/brew-1/unarchive'));
    expect(find.text(l10n.unarchivedMessage), findsOneWidget);
    expect(find.byTooltip(l10n.archiveButton), findsOneWidget);
  });

  testWidgets('抽出を登録し、抽出日時を UTC の ISO 8601 で送る', (tester) async {
    final l10n = await loadL10n();
    final api = await openBrewForm(tester);
    api.on('POST', '/api/brews', status: 200, body: brewJson(id: 'brew-9'));

    // 抽出日時の既定値は端末のタイムゾーンでの当日 (FR-11)。
    expect(find.widgetWithText(TextField, formatDay(today())), findsOneWidget);

    // 購入を選ぶ (必須。FR-11)。
    await scrollAndTap(tester, find.widgetWithText(ListTile, l10n.purchaseLabel));
    await tester.tap(find.text('Test Product').last);
    await tester.pumpAndSettle();

    await scrollAndEnterText(tester, find.byType(TextField).at(0), '2026-09-01');
    await scrollAndEnterText(tester, find.byType(TextField).at(1), '09:30');
    await scrollAndEnterText(tester, find.byType(TextField).at(2), '15.0');
    await scrollAndEnterText(tester, find.byType(TextField).at(3), '250');
    await scrollAndEnterText(tester, find.byType(TextField).at(4), '92.5');
    await scrollAndEnterText(tester, find.byType(TextField).at(5), '150');
    await scrollAndEnterText(tester, find.byType(TextField).at(6), 'V60');
    await scrollAndEnterText(tester, find.byType(TextField).at(7), 'Medium Fine');
    await scrollAndEnterText(tester, find.byType(TextField).at(8), 'Good');
    await scrollAndTap(tester, find.widgetWithText(FilledButton, l10n.saveButton));

    final body = api.lastBody('POST', '/api/brews')!;
    expect(body['purchase_id'], 'purchase-1');
    expect(body['dose_grams'], 15.0);
    expect(body['water_grams'], 250.0);
    expect(body['water_temp_c'], 92.5);
    expect(body['brew_time_seconds'], 150);
    expect(body['method'], 'V60');
    expect(body['grind_setting'], 'Medium Fine');
    expect(body['notes'], 'Good');
    // 端末のローカル時刻で入力した日時を、UTC の ISO 8601 で送る (FR-11)。
    final sent = body['brewed_at']! as String;
    expect(sent.endsWith('Z'), isTrue, reason: 'UTC の ISO 8601 にする');
    expect(DateTime.parse(sent).toLocal(), DateTime(2026, 9, 1, 9, 30));
    expect(find.text(l10n.savedMessage), findsOneWidget);
  });

  testWidgets('編集では抽出日時を端末のローカル時刻で表示する', (tester) async {
    final l10n = await loadL10n();
    final api = await openBrewForm(
      tester,
      id: 'brew-1',
      brew: brewJson(
        id: 'brew-1',
        brewedAt: '2026-09-01T00:30:00.000Z',
        rating: 3,
        purchase: purchaseJson(id: 'purchase-1'),
      ),
    );
    api.on('PATCH', '/api/brews/brew-1', status: 200, body: brewJson(id: 'brew-1'));

    // UTC の保存値を端末のローカル時刻にして入力欄に入れる (FR-11)。
    final local = parseUtcToLocal('2026-09-01T00:30:00.000Z');
    expect(find.widgetWithText(TextField, formatDay(local)), findsOneWidget);
    expect(find.widgetWithText(TextField, formatTime(local.hour, local.minute)), findsOneWidget);

    await scrollAndEnterText(tester, find.byType(TextField).at(5), '180');
    await scrollAndTap(tester, find.widgetWithText(FilledButton, l10n.saveButton));

    final body = api.lastBody('PATCH', '/api/brews/brew-1')!;
    expect(body['brew_time_seconds'], 180);
    expect(body['rating'], 3);
    expect(DateTime.parse(body['brewed_at']! as String).toLocal(), local);
  });

  testWidgets('購入を選ばないとエラーを表示し、API を呼ばない', (tester) async {
    final l10n = await loadL10n();
    final api = await openBrewForm(tester);

    await scrollAndTap(tester, find.widgetWithText(FilledButton, l10n.saveButton));

    expect(find.text(l10n.validationPurchase), findsOneWidget);
    expect(api.calls, <String>['GET /api/passkeys', 'GET /api/brews']);
  });

  testWidgets('日付と時刻と数の検証エラーを表示する', (tester) async {
    final l10n = await loadL10n();
    final api = await openBrewForm(tester);

    await scrollAndTap(tester, find.widgetWithText(ListTile, l10n.purchaseLabel));
    await tester.tap(find.text('Test Product').last);
    await tester.pumpAndSettle();
    await scrollAndEnterText(tester, find.byType(TextField).at(0), '2026-13-40');
    await scrollAndEnterText(tester, find.byType(TextField).at(1), '99:99');
    await scrollAndEnterText(tester, find.byType(TextField).at(2), '15.55');
    await scrollAndTap(tester, find.widgetWithText(FilledButton, l10n.saveButton));

    expect(find.text(l10n.validationDay), findsOneWidget);
    expect(find.text(l10n.validationTime), findsOneWidget);
    expect(find.text(l10n.validationDecimal), findsOneWidget);
    // 検証に失敗したときは API を呼ばない。
    expect(api.calls, <String>['GET /api/passkeys', 'GET /api/brews', 'GET /api/purchases']);
  });

  testWidgets('抽出方法と挽き目で候補を表示し、候補に無い値も入力できる', (tester) async {
    final api = await openBrewForm(tester);
    // 項目の並びは、日付、時刻、豆の量、湯量、湯の温度、時間、抽出方法、挽き目、感想 (FR-11)。
    const fields = <({String candidate, int index})>[
      (candidate: 'V60', index: 6),
      (candidate: 'Medium Fine', index: 7),
    ];
    for (final field in fields) {
      final textField = find.byType(TextField).at(field.index);
      // 候補の先頭の文字を入力する (API は前方一致で絞る。FR-13)。
      // 入力が候補そのものにならないよう、最後の 1 文字は入れない。
      await scrollAndEnterText(
        tester,
        textField,
        field.candidate.substring(0, field.candidate.length - 1),
      );
      await tester.pump(const Duration(milliseconds: 400));
      await tester.pumpAndSettle();
      // 過去の入力値が候補として表示される (FR-13)。
      expect(find.text(field.candidate), findsOneWidget, reason: field.candidate);

      // 候補に無い値もそのまま入力できる (FR-13)。
      final typed = 'No Candidate ${field.index}';
      await scrollAndEnterText(tester, textField, typed);
      expect(find.widgetWithText(TextField, typed), findsOneWidget);
    }
    expect(
      api.calls.where((call) => call.startsWith('GET /api/suggestions/')).length,
      greaterThanOrEqualTo(fields.length),
    );
  });

  testWidgets('入力の検証エラー (400) を表示する', (tester) async {
    final l10n = await loadL10n();
    final api = await openBrewForm(tester);
    api.on('POST', '/api/brews', status: 400, body: badRequestBody);

    await scrollAndTap(tester, find.widgetWithText(ListTile, l10n.purchaseLabel));
    await tester.tap(find.text('Test Product').last);
    await tester.pumpAndSettle();
    await scrollAndTap(tester, find.widgetWithText(FilledButton, l10n.saveButton));

    expect(find.text(l10n.errorValidation), findsOneWidget);
    expect(find.byType(BrewFormScreen), findsOneWidget);
  });

  testWidgets('認証の失効 (401) ではログイン画面へ遷移する', (tester) async {
    final l10n = await loadL10n();
    final api = await openBrewForm(tester);
    api.on('POST', '/api/brews', status: 401, body: unauthorizedBody);

    await scrollAndTap(tester, find.widgetWithText(ListTile, l10n.purchaseLabel));
    await tester.tap(find.text('Test Product').last);
    await tester.pumpAndSettle();
    await scrollAndTap(tester, find.widgetWithText(FilledButton, l10n.saveButton));

    expect(find.byType(LoginScreen), findsOneWidget);
  });

  testWidgets('抽出の一覧の行に端末のローカルの日時を表示する', (tester) async {
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(
          key: 'brews',
          items: <Map<String, Object?>>[
            brewJson(
              id: 'brew-1',
              brewedAt: '2026-09-01T00:30:00.000Z',
              purchase: purchaseJson(product: productJson(name: 'Ethiopia')),
            ),
          ],
        ),
      );
    onSuggestion(api, 'method', 'V60');
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());

    final l10n = await loadL10n();
    expect(
      find.text(
        l10n.brewRowSubtitleNoShop(displayTimestamp('2026-09-01T00:30:00.000Z', const Locale('en'))),
      ),
      findsOneWidget,
    );
  });

  testWidgets('編集の読み込みに失敗しても、再試行で回復する', (tester) async {
    final l10n = await loadL10n();
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      )
      ..onNetworkError('GET', '/api/brews/brew-1');
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());
    await openLocation(tester, AppRoutes.brewEditPath('brew-1'));

    expect(find.text(l10n.errorNetwork), findsOneWidget);

    // 再試行で読み込むと、失敗の表示が消えて値が入る。
    api.on(
      'GET',
      '/api/brews/brew-1',
      status: 200,
      body: brewJson(id: 'brew-1', notes: 'Retried Notes'),
    );
    await tester.tap(find.widgetWithText(TextButton, l10n.retryButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.errorNetwork), findsNothing);
    expect(find.widgetWithText(TextField, 'Retried Notes'), findsOneWidget);
  });
}
