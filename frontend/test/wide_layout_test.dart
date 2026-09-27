import 'package:brew_book/screens/brew_detail_screen.dart';
import 'package:brew_book/screens/brew_form_screen.dart';
import 'package:brew_book/screens/purchase_list_screen.dart';
import 'package:brew_book/widgets/wide_layout.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/fake_api.dart';
import 'support/fake_passkey_client.dart';
import 'support/fake_records.dart';
import 'support/pump_app.dart';

/// 幅 840 px 以上の 2 段組 (docs/design/components/WideLayout) のウィジェットテスト。
void main() {
  /// 幅 1280 px の画面でアプリを立ち上げる。
  Future<FakeApi> pumpWide(WidgetTester tester) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
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
              purchase: purchaseJson(product: productJson(name: 'Test Product')),
            ),
          ],
        ),
      )
      ..on('GET', '/api/brews/brew-1', status: 200, body: brewJson(id: 'brew-1'))
      ..on(
        'GET',
        '/api/purchases',
        status: 200,
        body: pageJson(key: 'purchases', items: <Map<String, Object?>>[]),
      );
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());
    return api;
  }

  testWidgets('幅 840 px 以上ではレールと一覧と詳細の 2 段組にする', (tester) async {
    final l10n = await loadL10n();
    await pumpWide(tester);

    // レール (88 px) と一覧の 2 段組にする (WideLayout のガイドライン)。
    expect(find.byType(WideLayout), findsOneWidget);
    expect(tester.getSize(find.byType(BrewbookNavigationRail)).width, 88);
    // 詳細は行を押すまで出さない。
    expect(find.byType(BrewDetailScreen), findsNothing);

    // 行を押すと右の面に詳細を出し、画面を積まない (ホームのメニューも置かない)。
    await tester.tap(find.text('Test Product'));
    await tester.pumpAndSettle();

    expect(find.byType(BrewDetailScreen), findsOneWidget);
    expect(find.byTooltip(l10n.menuTooltip), findsNothing);
    // 商品名は一覧の行、詳細の題、参照先のタイルに出る。
    expect(find.text('Test Product'), findsNWidgets(3));

    // 編集は右の面に開く (フォームも同じ位置に開く)。
    await tester.tap(find.byTooltip(l10n.editButton));
    await tester.pumpAndSettle();
    expect(find.byType(BrewFormScreen), findsOneWidget);

    // 閉じると詳細に戻る。
    await tester.tap(find.byType(CloseButton));
    await tester.pumpAndSettle();
    expect(find.byType(BrewDetailScreen), findsOneWidget);
  });

  testWidgets('レールから購入の一覧へ移れる', (tester) async {
    final l10n = await loadL10n();
    final api = await pumpWide(tester);

    await tester.tap(
      find.descendant(
        of: find.byType(BrewbookNavigationRail),
        matching: find.text(l10n.purchasesTitle),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.byType(PurchaseListScreen), findsOneWidget);
    expect(api.calls, contains('GET /api/purchases'));
  });

  testWidgets('幅 840 px 未満ではレールを出さず 1 列にする', (tester) async {
    await pumpWide(tester);

    // テストの画面は幅 800 px のため、レールは出ない (既定の幅)。
    tester.view.physicalSize = const Size(800, 600);
    await tester.pumpAndSettle();

    expect(find.byType(WideLayout), findsNothing);
    expect(find.byType(BrewbookNavigationRail), findsNothing);
  });
}
