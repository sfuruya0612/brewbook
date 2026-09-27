import 'package:brew_book/screens/brew_detail_screen.dart';
import 'package:brew_book/screens/home_screen.dart';
import 'package:brew_book/screens/login_screen.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/fake_api.dart';
import 'support/fake_passkey_client.dart';
import 'support/fake_records.dart';
import 'support/pump_app.dart';

void main() {
  testWidgets('起動時の確認が成功するとホームに抽出の一覧を表示する', (tester) async {
    final l10n = await loadL10n();
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
              brewedAt: '2026-09-01T01:00:00.000Z',
              rating: 4,
              purchase: purchaseJson(
                product: productJson(name: 'Ethiopia'),
                shop: shopJson(name: 'Test Shop'),
              ),
            ),
          ],
        ),
      );
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());

    expect(find.byType(HomeScreen), findsOneWidget);
    // 抽出の一覧 (カーソル方式の最初のページ) を読む (FR-11)。
    expect(find.text('Ethiopia'), findsOneWidget);
    expect(find.widgetWithText(FloatingActionButton, l10n.newBrewButton), findsOneWidget);
    // メニュー (購入、商品、店、統計、設定、ログアウト) を置く (ホームの AppBar)。
    expect(find.byTooltip(l10n.menuTooltip), findsOneWidget);
    expect(api.calls, <String>['GET /api/passkeys', 'GET /api/brews']);
  });

  testWidgets('抽出の一覧が空のときはその旨を表示する', (tester) async {
    final l10n = await loadL10n();
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      );
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());

    expect(find.text(l10n.noRecords), findsOneWidget);
  });

  testWidgets('一覧の行から抽出の詳細を開く', (tester) async {
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[brewJson(id: 'brew-1')]),
      )
      ..on('GET', '/api/brews/brew-1', status: 200, body: brewJson(id: 'brew-1'));
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());

    await tester.tap(find.text('Test Product'));
    await tester.pumpAndSettle();

    expect(find.byType(BrewDetailScreen), findsOneWidget);
  });

  testWidgets('アーカイブ済みを含める切り替えで読み直す', (tester) async {
    final api = signedInApi()
      ..onQuery('GET', '/api/brews', (query) {
        final includeArchived = query['include_archived'] == 'true';
        return (
          status: 200,
          body: pageJson(
            key: 'brews',
            items: <Map<String, Object?>>[
              brewJson(
                id: includeArchived ? 'brew-2' : 'brew-1',
                purchase: purchaseJson(
                  product: productJson(name: includeArchived ? 'Archived Brew' : 'Active Brew'),
                ),
                archivedAt: includeArchived ? '2026-09-02T00:00:00.000Z' : null,
              ),
            ],
          ),
        );
      });
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());

    expect(find.text('Active Brew'), findsOneWidget);
    expect(find.text('Archived Brew'), findsNothing);

    await tester.tap(find.byType(Switch));
    await tester.pumpAndSettle();

    expect(find.text('Archived Brew'), findsOneWidget);
    expect(api.calls, <String>['GET /api/passkeys', 'GET /api/brews', 'GET /api/brews']);
  });

  testWidgets('詳細から抽出をアーカイブすると一覧を読み直す', (tester) async {
    final l10n = await loadL10n();
    // 2 回目の一覧の読み込みでは、アーカイブ済みになった状態を返す (サーバーの状態を模す)。
    var listCalls = 0;
    final api = signedInApi()
      ..onQuery('GET', '/api/brews', (query) {
        listCalls += 1;
        return (
          status: 200,
          body: pageJson(
            key: 'brews',
            items: listCalls == 1
                ? <Map<String, Object?>>[brewJson(id: 'brew-1')]
                : <Map<String, Object?>>[],
          ),
        );
      })
      ..on('GET', '/api/brews/brew-1', status: 200, body: brewJson(id: 'brew-1'))
      ..on(
        'POST',
        '/api/brews/brew-1/archive',
        status: 200,
        body: brewJson(id: 'brew-1', archivedAt: '2026-09-02T00:00:00.000Z'),
      );
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());

    // アーカイブは詳細の画面から行う (一覧の行には置かない)。
    await tester.tap(find.text('Test Product'));
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip(l10n.archiveButton));
    await tester.pumpAndSettle();

    expect(api.calls, contains('POST /api/brews/brew-1/archive'));
    expect(api.calls.where((call) => call == 'GET /api/brews').length, 2);

    // 一覧へ戻ると、アーカイブ済みの抽出は出ない (FR-12)。
    await tester.pageBack();
    await tester.pumpAndSettle();
    expect(find.text(l10n.noRecords), findsOneWidget);
  });

  testWidgets('詳細から抽出のアーカイブ解除ができる', (tester) async {
    final l10n = await loadL10n();
    // アーカイブ解除の後は、解除済みの抽出を返す (サーバーの状態を模す)。
    var unarchived = false;
    final api = signedInApi()
      ..onQuery('GET', '/api/brews', (query) {
        final includeArchived = query['include_archived'] == 'true';
        if (!includeArchived) {
          return (
            status: 200,
            body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
          );
        }
        return (
          status: 200,
          body: pageJson(
            key: 'brews',
            items: <Map<String, Object?>>[
              brewJson(
                id: 'brew-1',
                purchase: purchaseJson(
                  product: productJson(name: unarchived ? 'Active' : 'Old'),
                ),
                archivedAt: unarchived ? null : '2026-09-02T00:00:00.000Z',
              ),
            ],
          ),
        );
      })
      ..onQuery('GET', '/api/brews/brew-1', (query) {
        return (
          status: 200,
          body: brewJson(
            id: 'brew-1',
            purchase: purchaseJson(
              product: productJson(name: unarchived ? 'Active' : 'Old'),
            ),
            archivedAt: unarchived ? null : '2026-09-02T00:00:00.000Z',
          ),
        );
      })
      ..onQuery('POST', '/api/brews/brew-1/unarchive', (query) {
        unarchived = true;
        return (status: 200, body: brewJson(id: 'brew-1'));
      });
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());

    await tester.tap(find.byType(Switch));
    await tester.pumpAndSettle();
    expect(find.text('Old'), findsOneWidget);

    // アーカイブ済みの抽出は、詳細からアーカイブ解除できる (FR-12)。
    await tester.tap(find.text('Old'));
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip(l10n.unarchiveButton));
    await tester.pumpAndSettle();

    expect(api.calls, contains('POST /api/brews/brew-1/unarchive'));
    expect(find.text(l10n.unarchivedMessage), findsOneWidget);

    await tester.pageBack();
    await tester.pumpAndSettle();
    expect(find.text('Active'), findsOneWidget);
  });

  testWidgets('起動時の確認がネットワークエラーなら再試行を促す', (tester) async {
    final l10n = await loadL10n();
    final api = FakeApi()..onNetworkError('GET', '/api/passkeys');
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());

    // ログイン状態が分からないため、ログイン画面へは遷移させない。
    expect(find.byType(LoginScreen), findsNothing);
    expect(find.text(l10n.errorNetwork), findsOneWidget);
    expect(find.widgetWithText(TextButton, l10n.retryButton), findsOneWidget);
  });

  testWidgets('起動時の確認が 401 以外の API エラーなら再試行を促す', (tester) async {
    final l10n = await loadL10n();
    // 500 はセッションの失効を意味しないため、ログイン画面へは遷移させない。
    final api = FakeApi()
      ..on('GET', '/api/passkeys', status: 500, body: <String, Object?>{
        'error': <String, Object?>{'code': 'internal_error', 'message': 'temporary'},
      });
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());

    expect(find.byType(LoginScreen), findsNothing);
    expect(find.text(l10n.errorUnexpected), findsOneWidget);
    expect(find.widgetWithText(TextButton, l10n.retryButton), findsOneWidget);
  });

  testWidgets('再試行でホームを表示する', (tester) async {
    final l10n = await loadL10n();
    final api = FakeApi()..onNetworkError('GET', '/api/passkeys');
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());

    api
      ..on('GET', '/api/passkeys', status: 200, body: <String, Object?>{'passkeys': <Object?>[]})
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      );
    await tester.tap(find.widgetWithText(TextButton, l10n.retryButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.noRecords), findsOneWidget);
    expect(api.calls, <String>['GET /api/passkeys', 'GET /api/passkeys', 'GET /api/brews']);
  });

  testWidgets('メニューから購入の一覧と商品の一覧と店の一覧へ遷移する', (tester) async {
    final l10n = await loadL10n();
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      )
      ..on(
        'GET',
        '/api/purchases',
        status: 200,
        body: pageJson(key: 'purchases', items: <Map<String, Object?>>[purchaseJson()]),
      )
      ..on(
        'GET',
        '/api/products',
        status: 200,
        body: pageJson(key: 'products', items: <Map<String, Object?>>[productJson()]),
      )
      ..on(
        'GET',
        '/api/shops',
        status: 200,
        body: pageJson(key: 'shops', items: <Map<String, Object?>>[shopJson()]),
      );
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());

    // メニューはホームにだけ置くため、一覧を開いた後はホームへ戻る。
    await _openMenu(tester, l10n.menuTooltip, l10n.purchasesTitle);
    expect(find.text(l10n.newPurchaseButton), findsOneWidget);
    await tester.pageBack();
    await tester.pumpAndSettle();

    await _openMenu(tester, l10n.menuTooltip, l10n.productsTitle);
    expect(find.text(l10n.newProductButton), findsOneWidget);
    await tester.pageBack();
    await tester.pumpAndSettle();

    await _openMenu(tester, l10n.menuTooltip, l10n.shopsTitle);
    expect(find.text(l10n.newShopButton), findsOneWidget);
  });

  testWidgets('ログアウトするとログイン画面へ遷移する', (tester) async {
    final l10n = await loadL10n();
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      )
      ..on('POST', '/api/auth/logout', status: 200, body: <String, Object?>{});
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());

    // ログアウトはメニューの区切りの下に置く (ホームの AppBar)。
    await _openMenu(tester, l10n.menuTooltip, l10n.logoutButton);

    expect(find.byType(LoginScreen), findsOneWidget);
    expect(api.calls, <String>[
      'GET /api/passkeys',
      'GET /api/brews',
      'POST /api/auth/logout',
    ]);
  });

  testWidgets('ログアウトがネットワークエラーなら再試行を促す', (tester) async {
    final l10n = await loadL10n();
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      );
    api.onNetworkError('POST', '/api/auth/logout');
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());

    await _openMenu(tester, l10n.menuTooltip, l10n.logoutButton);

    expect(find.text(l10n.errorNetwork), findsOneWidget);
    expect(find.byType(HomeScreen), findsOneWidget);
  });
}

/// メニューを開いて項目を選ぶ。
Future<void> _openMenu(WidgetTester tester, String tooltip, String item) async {
  await tester.tap(find.byTooltip(tooltip));
  await tester.pumpAndSettle();
  await tester.tap(find.text(item).last);
  await tester.pumpAndSettle();
}
