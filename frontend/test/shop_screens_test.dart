import 'package:brew_book/router/app_router.dart';
import 'package:brew_book/screens/home_screen.dart';
import 'package:brew_book/screens/login_screen.dart';
import 'package:brew_book/screens/shop_form_screen.dart';
import 'package:brew_book/screens/shop_list_screen.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/fake_api.dart';
import 'support/fake_passkey_client.dart';
import 'support/fake_records.dart';
import 'support/pump_app.dart';

/// 店の画面 (一覧、登録、編集、アーカイブ。FR-6、FR-12) のウィジェットテスト。
void main() {
  /// 店の一覧の画面を開いた状態にする。
  Future<FakeApi> openShopList(
    WidgetTester tester, {
    required Map<String, Object?> listBody,
  }) async {
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      )
      ..on('GET', '/api/shops', status: 200, body: listBody);
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());
    await openLocation(tester, AppRoutes.shops);
    expect(find.byType(ShopListScreen), findsOneWidget);
    return api;
  }

  /// 店の登録の画面を開いた状態にする。
  Future<FakeApi> openShopForm(WidgetTester tester, {String? id}) async {
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      )
      ..on(
        'GET',
        '/api/shops/shop-1',
        status: 200,
        body: shopJson(id: 'shop-1', name: 'Old Shop', address: 'Old Address'),
      );
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());
    await openLocation(tester, id == null ? AppRoutes.shopNew : AppRoutes.shopEditPath(id));
    expect(find.byType(ShopFormScreen), findsOneWidget);
    return api;
  }

  testWidgets('一覧に行を表示し、行から編集を開く', (tester) async {
    final l10n = await loadL10n();
    final api = await openShopList(
      tester,
      listBody: pageJson(
        key: 'shops',
        items: <Map<String, Object?>>[
          shopJson(id: 'shop-1', name: 'First Shop', address: 'First Address'),
          shopJson(id: 'shop-2', name: 'Second Shop'),
        ],
      ),
    );
    // 編集の画面は現在の値を読む (FR-6)。
    api.on(
      'GET',
      '/api/shops/shop-1',
      status: 200,
      body: shopJson(id: 'shop-1', name: 'First Shop', address: 'First Address'),
    );

    expect(find.text('First Shop'), findsOneWidget);
    expect(find.text('First Address'), findsOneWidget);
    expect(find.text('Second Shop'), findsOneWidget);
    expect(find.widgetWithText(FloatingActionButton, l10n.newShopButton), findsOneWidget);

    await tester.tap(find.text('First Shop'));
    await tester.pumpAndSettle();

    expect(find.byType(ShopFormScreen), findsOneWidget);
    // 編集では現在の値を読み込む (FR-6)。
    expect(find.widgetWithText(TextField, 'First Shop'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'First Address'), findsOneWidget);
    expect(api.calls, <String>[
      'GET /api/passkeys',
      'GET /api/brews',
      'GET /api/shops',
      'GET /api/shops/shop-1',
    ]);
  });

  testWidgets('一覧の切り替えでアーカイブ済みを表示し、行からアーカイブと解除ができる', (tester) async {
    final l10n = await loadL10n();
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      )
      // アーカイブ済みは include_archived=true のときだけ返す (FR-12)。
      ..onQuery('GET', '/api/shops', (query) {
        final includeArchived = query['include_archived'] == 'true';
        return (
          status: 200,
          body: pageJson(
            key: 'shops',
            items: <Map<String, Object?>>[
              shopJson(id: 'shop-1', name: 'Active Shop'),
              if (includeArchived)
                shopJson(id: 'shop-2', name: 'Old Shop', archivedAt: '2026-09-02T00:00:00.000Z'),
            ],
          ),
        );
      })
      // 編集の画面 (詳細を兼ねる) が読む API (FR-6)。
      ..on('GET', '/api/shops/shop-1', status: 200, body: shopJson(id: 'shop-1', name: 'Active Shop'))
      ..on(
        'GET',
        '/api/shops/shop-2',
        status: 200,
        body: shopJson(id: 'shop-2', name: 'Old Shop', archivedAt: '2026-09-02T00:00:00.000Z'),
      )
      ..on(
        'POST',
        '/api/shops/shop-1/archive',
        status: 200,
        body: shopJson(id: 'shop-1', archivedAt: '2026-09-03T00:00:00.000Z'),
      )
      ..on(
        'POST',
        '/api/shops/shop-2/unarchive',
        status: 200,
        body: shopJson(id: 'shop-2', name: 'Old Shop'),
      );
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());
    await openLocation(tester, AppRoutes.shops);

    // 既定ではアーカイブ済みを返さない (API の既定)。
    expect(find.text('Active Shop'), findsOneWidget);
    expect(find.text('Old Shop'), findsNothing);

    await tester.tap(find.byType(Switch));
    await tester.pumpAndSettle();
    expect(find.text('Old Shop'), findsOneWidget);
    // アーカイブ済みの行はバッジで区別する (ListRow のガイドライン)。
    expect(find.text(l10n.archivedBadge), findsOneWidget);

    // アーカイブは編集の画面 (詳細を兼ねる) の AppBar から行う。
    await tester.tap(find.text('Active Shop'));
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip(l10n.archiveButton));
    await tester.pumpAndSettle();
    expect(api.calls, contains('POST /api/shops/shop-1/archive'));

    await tester.tap(find.byType(CloseButton));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Old Shop'));
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip(l10n.unarchiveButton));
    await tester.pumpAndSettle();
    expect(api.calls, contains('POST /api/shops/shop-2/unarchive'));
  });

  testWidgets('店を登録する', (tester) async {
    final l10n = await loadL10n();
    final api = await openShopForm(tester);
    api.on('POST', '/api/shops', status: 200, body: shopJson(name: 'New Shop'));

    await tester.enterText(find.byType(TextField).at(0), 'New Shop');
    await tester.enterText(find.byType(TextField).at(1), 'New Address');
    await tester.tap(find.widgetWithText(TextButton, l10n.saveButton));
    await tester.pumpAndSettle();

    expect(api.lastBody('POST', '/api/shops'), <String, Object?>{
      'name': 'New Shop',
      'address': 'New Address',
    });
    expect(find.text(l10n.savedMessage), findsOneWidget);
    // 保存するとホームへ戻り、一覧を読み直す (ホームの下の一覧も含む)。
    expect(find.byType(HomeScreen), findsOneWidget);
  });

  testWidgets('店名が空ならエラーを表示し、API を呼ばない', (tester) async {
    final l10n = await loadL10n();
    final api = await openShopForm(tester);

    await tester.enterText(find.byType(TextField).at(0), '   ');
    await tester.tap(find.widgetWithText(TextButton, l10n.saveButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.validationRequired), findsOneWidget);
    expect(api.calls, <String>['GET /api/passkeys', 'GET /api/brews']);
  });

  testWidgets('店を編集し、住所を空にすると null を送る', (tester) async {
    final l10n = await loadL10n();
    final api = await openShopForm(tester, id: 'shop-1');
    api.on('PATCH', '/api/shops/shop-1', status: 200, body: shopJson(name: 'Renamed'));

    await tester.enterText(find.byType(TextField).at(0), 'Renamed');
    await tester.enterText(find.byType(TextField).at(1), '');
    await tester.tap(find.widgetWithText(TextButton, l10n.saveButton));
    await tester.pumpAndSettle();

    expect(api.lastBody('PATCH', '/api/shops/shop-1'), <String, Object?>{
      'name': 'Renamed',
      'address': null,
    });
    expect(find.text(l10n.savedMessage), findsOneWidget);
  });

  testWidgets('入力の検証エラー (400) を表示する', (tester) async {
    final l10n = await loadL10n();
    final api = await openShopForm(tester);
    api.on('POST', '/api/shops', status: 400, body: badRequestBody);

    await tester.enterText(find.byType(TextField).at(0), 'New Shop');
    await tester.tap(find.widgetWithText(TextButton, l10n.saveButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.errorValidation), findsOneWidget);
    expect(find.byType(ShopFormScreen), findsOneWidget);
  });

  testWidgets('認証の失効 (401) ではログイン画面へ遷移する', (tester) async {
    final l10n = await loadL10n();
    final api = await openShopForm(tester);
    api.on('POST', '/api/shops', status: 401, body: unauthorizedBody);

    await tester.enterText(find.byType(TextField).at(0), 'New Shop');
    await tester.tap(find.widgetWithText(TextButton, l10n.saveButton));
    await tester.pumpAndSettle();

    expect(find.byType(LoginScreen), findsOneWidget);
  });

  testWidgets('一覧の読み込みに失敗すると再試行を促す', (tester) async {
    final l10n = await loadL10n();
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      );
    api.onNetworkError('GET', '/api/shops');
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());
    await openLocation(tester, AppRoutes.shops);

    expect(find.text(l10n.errorNetwork), findsOneWidget);
    expect(find.widgetWithText(TextButton, l10n.retryButton), findsOneWidget);

    api.on(
      'GET',
      '/api/shops',
      status: 200,
      body: pageJson(key: 'shops', items: <Map<String, Object?>>[shopJson(name: 'Retried')]),
    );
    await tester.tap(find.widgetWithText(TextButton, l10n.retryButton));
    await tester.pumpAndSettle();

    expect(find.text('Retried'), findsOneWidget);
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
      ..onNetworkError('GET', '/api/shops/shop-1');
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());
    await openLocation(tester, AppRoutes.shopEditPath('shop-1'));

    expect(find.text(l10n.errorNetwork), findsOneWidget);

    // 再試行で読み込むと、失敗の表示が消えて値が入る。
    api.on(
      'GET',
      '/api/shops/shop-1',
      status: 200,
      body: shopJson(id: 'shop-1', name: 'Retried Shop', address: 'Retried Address'),
    );
    await tester.tap(find.widgetWithText(TextButton, l10n.retryButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.errorNetwork), findsNothing);
    expect(find.widgetWithText(TextField, 'Retried Shop'), findsOneWidget);
  });
}
