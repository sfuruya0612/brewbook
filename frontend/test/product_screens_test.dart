import 'package:brew_book/router/app_router.dart';
import 'package:brew_book/screens/login_screen.dart';
import 'package:brew_book/screens/product_form_screen.dart';
import 'package:brew_book/screens/product_list_screen.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/fake_api.dart';
import 'support/fake_passkey_client.dart';
import 'support/fake_records.dart';
import 'support/pump_app.dart';

/// 商品の画面 (一覧、登録、編集、アーカイブ、タグ。FR-7、FR-8、FR-12) のウィジェットテスト。
void main() {
  /// 商品の一覧の画面を開いた状態にする。
  Future<FakeApi> openProductList(WidgetTester tester, {required Map<String, Object?> listBody}) async {
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      )
      ..on('GET', '/api/products', status: 200, body: listBody);
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());
    await openLocation(tester, AppRoutes.products);
    expect(find.byType(ProductListScreen), findsOneWidget);
    return api;
  }

  /// 商品の登録と編集の画面を開いた状態にする。
  Future<FakeApi> openProductForm(WidgetTester tester, {String? id}) async {
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      )
      // サジェストの API は候補を返す (FR-13)。
      ..on(
        'GET',
        '/api/products/product-1',
        status: 200,
        body: productJson(
          id: 'product-1',
          name: 'Old Product',
          flavorNotes: <String>['Fruity', 'Sweet'],
        ),
      );
    onSuggestion(api, 'producer', 'Producer A');
    onSuggestion(api, 'origin', 'Origin A');
    onSuggestion(api, 'region', 'Region A');
    onSuggestion(api, 'process', 'Process A');
    onSuggestion(api, 'variety', 'Variety A');
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());
    await openLocation(tester, id == null ? AppRoutes.productNew : AppRoutes.productEditPath(id));
    expect(find.byType(ProductFormScreen), findsOneWidget);
    return api;
  }

  testWidgets('一覧に行を表示し、行から編集を開く', (tester) async {
    final l10n = await loadL10n();
    final api = await openProductList(
      tester,
      listBody: pageJson(
        key: 'products',
        items: <Map<String, Object?>>[
          productJson(id: 'product-1', name: 'Ethiopia', producer: 'Producer A'),
        ],
      ),
    );
    api.on(
      'GET',
      '/api/products/product-1',
      status: 200,
      body: productJson(id: 'product-1', name: 'Ethiopia', producer: 'Producer A'),
    );

    expect(find.text('Ethiopia'), findsOneWidget);
    expect(find.text('Producer A'), findsOneWidget);
    expect(find.widgetWithText(FloatingActionButton, l10n.newProductButton), findsOneWidget);

    await tester.tap(find.text('Ethiopia'));
    await tester.pumpAndSettle();

    expect(find.byType(ProductFormScreen), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Ethiopia'), findsOneWidget);
    expect(api.calls, contains('GET /api/products/product-1'));
  });

  testWidgets('一覧の行からアーカイブとアーカイブ解除ができる', (tester) async {
    final l10n = await loadL10n();
    final api = await openProductList(
      tester,
      listBody: pageJson(
        key: 'products',
        items: <Map<String, Object?>>[
          productJson(id: 'product-1', name: 'Ethiopia'),
          productJson(id: 'product-2', name: 'Kenya', archivedAt: '2026-09-02T00:00:00.000Z'),
        ],
      ),
    );
    api
      ..on(
        'POST',
        '/api/products/product-1/archive',
        status: 200,
        body: productJson(id: 'product-1', archivedAt: '2026-09-03T00:00:00.000Z'),
      )
      ..on(
        'POST',
        '/api/products/product-2/unarchive',
        status: 200,
        body: productJson(id: 'product-2', name: 'Kenya'),
      );

    await tester.tap(find.byTooltip(l10n.archiveButton));
    await tester.pumpAndSettle();
    expect(api.calls, contains('POST /api/products/product-1/archive'));

    await tester.tap(find.byTooltip(l10n.unarchiveButton));
    await tester.pumpAndSettle();
    expect(api.calls, contains('POST /api/products/product-2/unarchive'));
  });

  testWidgets('商品を登録する (タグを含む)', (tester) async {
    final l10n = await loadL10n();
    final api = await openProductForm(tester);
    api.on('POST', '/api/products', status: 200, body: productJson(name: 'New Product'));

    await scrollAndEnterText(tester, find.byType(TextField).at(0), 'New Product');
    await scrollAndEnterText(tester, find.byType(TextField).at(1), 'Producer A');
    // Flavor Notes のタグを追加する (FR-8)。
    await scrollAndEnterText(tester, find.byType(TextField).at(6), 'Fruity');
    await scrollAndTap(tester, find.widgetWithText(FilledButton, l10n.addButton));
    expect(find.widgetWithText(InputChip, 'Fruity'), findsOneWidget);

    await scrollAndTap(tester, find.widgetWithText(FilledButton, l10n.saveButton));

    expect(api.lastBody('POST', '/api/products'), <String, Object?>{
      'name': 'New Product',
      'producer': 'Producer A',
      'origin': null,
      'region': null,
      'process': null,
      'variety': null,
      'flavor_notes': <String>['Fruity'],
    });
    expect(find.text(l10n.savedMessage), findsOneWidget);
  });

  testWidgets('商品を編集し、タグは入力した配列で置き換える', (tester) async {
    final l10n = await loadL10n();
    final api = await openProductForm(tester, id: 'product-1');
    api.on('PATCH', '/api/products/product-1', status: 200, body: productJson(name: 'Old Product'));

    // 読み込んだタグが表示される (FR-8)。
    expect(find.widgetWithText(InputChip, 'Fruity'), findsOneWidget);
    expect(find.widgetWithText(InputChip, 'Sweet'), findsOneWidget);

    // 1 つ外し、1 つ足して保存する。
    await scrollAndTap(
      tester,
      find.descendant(
        of: find.widgetWithText(InputChip, 'Fruity'),
        matching: find.byTooltip(l10n.deleteButton),
      ),
    );
    await scrollAndEnterText(tester, find.byType(TextField).at(6), 'Floral');
    await scrollAndTap(tester, find.widgetWithText(FilledButton, l10n.addButton));
    await scrollAndTap(tester, find.widgetWithText(FilledButton, l10n.saveButton));

    expect(api.lastBody('PATCH', '/api/products/product-1'), <String, Object?>{
      'name': 'Old Product',
      'producer': null,
      'origin': null,
      'region': null,
      'process': null,
      'variety': null,
      'flavor_notes': <String>['Sweet', 'Floral'],
    });
  });

  testWidgets('商品名が空ならエラーを表示し、API を呼ばない', (tester) async {
    final l10n = await loadL10n();
    final api = await openProductForm(tester);

    await scrollAndTap(tester, find.widgetWithText(FilledButton, l10n.saveButton));

    expect(find.text(l10n.validationRequired), findsOneWidget);
    expect(api.calls, <String>['GET /api/passkeys', 'GET /api/brews']);
  });

  testWidgets('自由記述の 5 項目で候補を表示し、候補に無い値も入力できる', (tester) async {
    final api = await openProductForm(tester);
    // 項目の並びは、名前、Producer、Origin、Region、Process、Variety、タグ (FR-7、FR-8)。
    const fields = <({String candidate, int index})>[
      (candidate: 'Producer A', index: 1),
      (candidate: 'Origin A', index: 2),
      (candidate: 'Region A', index: 3),
      (candidate: 'Process A', index: 4),
      (candidate: 'Variety A', index: 5),
    ];
    for (final field in fields) {
      final textField = find.byType(TextField).at(field.index);
      // 候補の先頭の文字を入力する (API は前方一致で絞る。FR-13)。
      await scrollAndEnterText(tester, textField, field.candidate.substring(0, 3));
      await tester.pump(const Duration(milliseconds: 400));
      await tester.pumpAndSettle();
      // 過去の入力値が候補として表示される (FR-13)。
      expect(find.text(field.candidate), findsOneWidget, reason: field.candidate);
      // 候補に無い値もそのまま入力できる (FR-13)。
      final typed = 'No Candidate ${field.index}';
      await scrollAndEnterText(tester, textField, typed);
      expect(find.widgetWithText(TextField, typed), findsOneWidget);
      expect(find.text(field.candidate), findsNothing);
    }
    expect(
      api.calls.where((call) => call.startsWith('GET /api/suggestions/')).length,
      greaterThanOrEqualTo(fields.length),
    );
  });

  testWidgets('入力の検証エラー (400) を表示する', (tester) async {
    final l10n = await loadL10n();
    final api = await openProductForm(tester);
    api.on('POST', '/api/products', status: 400, body: badRequestBody);

    await scrollAndEnterText(tester, find.byType(TextField).at(0), 'New Product');
    await scrollAndTap(tester, find.widgetWithText(FilledButton, l10n.saveButton));

    expect(find.text(l10n.errorValidation), findsOneWidget);
    expect(find.byType(ProductFormScreen), findsOneWidget);
  });

  testWidgets('認証の失効 (401) ではログイン画面へ遷移する', (tester) async {
    final l10n = await loadL10n();
    final api = await openProductForm(tester);
    api.on('POST', '/api/products', status: 401, body: unauthorizedBody);

    await scrollAndEnterText(tester, find.byType(TextField).at(0), 'New Product');
    await scrollAndTap(tester, find.widgetWithText(FilledButton, l10n.saveButton));

    expect(find.byType(LoginScreen), findsOneWidget);
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
      ..onNetworkError('GET', '/api/products/product-1');
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());
    await openLocation(tester, AppRoutes.productEditPath('product-1'));

    expect(find.text(l10n.errorNetwork), findsOneWidget);

    // 再試行で読み込むと、失敗の表示が消えて値が入る。
    api.on(
      'GET',
      '/api/products/product-1',
      status: 200,
      body: productJson(id: 'product-1', name: 'Retried Product'),
    );
    await tester.tap(find.widgetWithText(TextButton, l10n.retryButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.errorNetwork), findsNothing);
    expect(find.widgetWithText(TextField, 'Retried Product'), findsOneWidget);
  });
}
