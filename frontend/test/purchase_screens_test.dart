import 'dart:typed_data';

import 'package:brew_book/photo/image_converter.dart';
import 'package:brew_book/photo/photo_picker.dart';
import 'package:brew_book/records/values.dart';
import 'package:brew_book/router/app_router.dart';
import 'package:brew_book/screens/login_screen.dart';
import 'package:brew_book/screens/product_form_screen.dart';
import 'package:brew_book/screens/purchase_detail_screen.dart';
import 'package:brew_book/screens/purchase_form_screen.dart';
import 'package:brew_book/screens/purchase_list_screen.dart';
import 'package:brew_book/screens/shop_form_screen.dart';
import 'package:fl_chart/fl_chart.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/fake_api.dart';
import 'support/fake_passkey_client.dart';
import 'support/fake_photo.dart';
import 'support/fake_records.dart';
import 'support/pump_app.dart';

/// 推測できない応答 (FR-19)。写真を選ぶテストのうち、推測の値を使わないテストが登録する。
Map<String, Object?> emptySuggestionJson() {
  return <String, Object?>{
    'product': null,
    'roast': null,
    'roast_date': null,
    'price_amount': null,
    'weight_grams': null,
  };
}

/// 購入の画面 (一覧、詳細、登録、編集、アーカイブ、写真。FR-9、FR-10、FR-12) と、
/// 写真からの推測の反映 (FR-19) のウィジェットテスト。
void main() {
  /// 購入の一覧の画面を開いた状態にする。
  Future<FakeApi> openPurchaseList(
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
      ..on('GET', '/api/purchases', status: 200, body: listBody);
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());
    await openLocation(tester, AppRoutes.purchases);
    expect(find.byType(PurchaseListScreen), findsOneWidget);
    return api;
  }

  /// 購入の詳細の画面を開いた状態にする。
  Future<FakeApi> openPurchaseDetail(
    WidgetTester tester, {
    Map<String, Object?>? purchase,
    List<Map<String, Object?>> ratings = const <Map<String, Object?>>[],
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
        '/api/purchases/purchase-1',
        status: 200,
        body: purchase ?? purchaseJson(id: 'purchase-1'),
      )
      // 購入ごとの評価の推移 (FR-18)。
      ..on(
        'GET',
        '/api/purchases/purchase-1/rating-history',
        status: 200,
        body: <String, Object?>{'ratings': ratings},
      );
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());
    await openLocation(tester, AppRoutes.purchasePath('purchase-1'));
    expect(find.byType(PurchaseDetailScreen), findsOneWidget);
    return api;
  }

  /// 購入の登録と編集の画面を開いた状態にする。
  Future<FakeApi> openPurchaseForm(
    WidgetTester tester, {
    String? id,
    Map<String, Object?>? purchase,
    PhotoPicker? picker,
    FakeImageConverter? converter,
    FakeUploadClient? upload,
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
        '/api/purchases/purchase-1',
        status: 200,
        body: purchase ?? purchaseJson(id: 'purchase-1', purchasedOn: '2026-08-01'),
      )
      // 参照の選択のダイアログは、カーソル方式の一覧を読む (FR-9)。
      ..on(
        'GET',
        '/api/products',
        status: 200,
        body: pageJson(
          key: 'products',
          items: <Map<String, Object?>>[productJson(id: 'product-1', name: 'Ethiopia')],
        ),
      )
      ..on(
        'GET',
        '/api/shops',
        status: 200,
        body: pageJson(
          key: 'shops',
          items: <Map<String, Object?>>[shopJson(id: 'shop-1', name: 'Test Shop')],
        ),
      );
    onSuggestion(api, 'roast', 'Medium');
    await pumpApp(
      tester,
      apiClient: api.client(),
      passkeyClient: FakePasskeyClient(),
      photoPicker: picker,
      imageConverter: converter,
      photoUploadClient: upload?.client(),
    );
    await openLocation(tester, id == null ? AppRoutes.purchaseNew : AppRoutes.purchaseEditPath(id));
    expect(find.byType(PurchaseFormScreen), findsOneWidget);
    return api;
  }

  /// 選択のダイアログから記録を 1 件選ぶ。
  ///
  /// ピッカーは枠の中の項目名 (「商品」「店」「購入」) を押すと開く。
  Future<void> pickFromDialog(WidgetTester tester, String label, String value) async {
    await scrollAndTap(tester, find.text(label));
    await tester.tap(find.text(value).last);
    await tester.pumpAndSettle();
  }

  testWidgets('一覧に行を表示し、行から詳細を開く', (tester) async {
    final l10n = await loadL10n();
    final api = await openPurchaseList(
      tester,
      listBody: pageJson(
        key: 'purchases',
        items: <Map<String, Object?>>[
          purchaseJson(id: 'purchase-1', purchasedOn: '2026-09-01'),
        ],
      ),
    );
    api.on('GET', '/api/purchases/purchase-1', status: 200, body: purchaseJson(id: 'purchase-1'));

    expect(find.text('9/1/2026'), findsOneWidget);
    expect(find.widgetWithText(FloatingActionButton, l10n.newPurchaseButton), findsOneWidget);

    await tester.tap(find.text('9/1/2026'));
    await tester.pumpAndSettle();

    expect(find.byType(PurchaseDetailScreen), findsOneWidget);
    expect(api.calls, contains('GET /api/purchases/purchase-1'));
  });

  testWidgets('詳細からアーカイブとアーカイブ解除ができる', (tester) async {
    final l10n = await loadL10n();
    final api = await openPurchaseList(
      tester,
      listBody: pageJson(
        key: 'purchases',
        items: <Map<String, Object?>>[
          purchaseJson(id: 'purchase-1', purchasedOn: '2026-09-01'),
        ],
      ),
    );
    // アーカイブの状態をサーバーのように覚え、読み直しで返す。
    var archived = false;
    api
      ..onQuery('GET', '/api/purchases/purchase-1', (query) {
        return (
          status: 200,
          body: purchaseJson(
            id: 'purchase-1',
            archivedAt: archived ? '2026-09-03T00:00:00.000Z' : null,
          ),
        );
      })
      ..on(
        'GET',
        '/api/purchases/purchase-1/rating-history',
        status: 200,
        body: <String, Object?>{'ratings': <Object?>[]},
      )
      ..onQuery('POST', '/api/purchases/purchase-1/archive', (query) {
        archived = true;
        return (
          status: 200,
          body: purchaseJson(id: 'purchase-1', archivedAt: '2026-09-03T00:00:00.000Z'),
        );
      })
      ..onQuery('POST', '/api/purchases/purchase-1/unarchive', (query) {
        archived = false;
        return (status: 200, body: purchaseJson(id: 'purchase-1'));
      });

    // アーカイブは詳細の画面から行う (一覧の行には置かない)。
    await tester.tap(find.text('9/1/2026'));
    await tester.pumpAndSettle();
    expect(find.byType(PurchaseDetailScreen), findsOneWidget);

    await tester.tap(find.byTooltip(l10n.archiveButton));
    await tester.pumpAndSettle();

    expect(api.calls, contains('POST /api/purchases/purchase-1/archive'));
    expect(find.text(l10n.archivedMessage), findsOneWidget);
    // 通知が消えるまで進める (次の通知が待ち行列に残らないようにする)。
    await tester.pump(const Duration(seconds: 5));
    await tester.pumpAndSettle();

    // アーカイブ済みの購入は、アーカイブ解除で戻せる (FR-12)。
    await tester.tap(find.byTooltip(l10n.unarchiveButton));
    await tester.pumpAndSettle();

    expect(api.calls, contains('POST /api/purchases/purchase-1/unarchive'));
    expect(find.text(l10n.unarchivedMessage), findsOneWidget);
  });

  testWidgets('詳細で商品と店をたどり、写真と評価の推移の場所を表示する', (tester) async {
    final l10n = await loadL10n();
    final api = await openPurchaseDetail(
      tester,
      purchase: purchaseJson(
        id: 'purchase-1',
        photoKey: 'users/user-1/purchases/purchase-1/photo.jpg',
        priceAmount: 1200,
        priceCurrency: 'JPY',
        weightGrams: 200,
        roast: 'Medium',
        product: productJson(id: 'product-1', name: 'Ethiopia'),
        shop: shopJson(id: 'shop-1', name: 'Test Shop'),
      ),
    );
    // 参照をたどる先の画面が読む API (UC-6)。
    api
      ..on('GET', '/api/products/product-1', status: 200, body: productJson(id: 'product-1'))
      ..on('GET', '/api/shops/shop-1', status: 200, body: shopJson(id: 'shop-1'))
      ..on(
        'GET',
        '/api/purchases/purchase-1',
        status: 200,
        body: purchaseJson(id: 'purchase-1'),
      );

    // 商品名は題と参照先のタイルの両方に出る (Detail のガイドライン)。
    expect(find.text('Ethiopia'), findsNWidgets(2));
    expect(find.text('Test Shop'), findsNWidgets(2));
    expect(find.text(l10n.priceValue('1200', 'JPY')), findsOneWidget);
    expect(find.text(l10n.gramsValue('200')), findsOneWidget);
    expect(find.byKey(const Key('purchase-photo')), findsOneWidget);
    // 評価の推移の折れ線グラフ (FR-18)。評価が無ければ記録が無い旨を出す。
    expect(find.byKey(const Key('purchase-rating-history-chart')), findsNothing);
    expect(find.text(l10n.noRecords), findsOneWidget);

    await scrollAndTap(tester, find.text('Ethiopia').last);
    expect(find.byType(ProductFormScreen), findsOneWidget);

    // フォームは閉じる (x) で戻る (AppBar のガイドライン)。
    await tester.tap(find.byType(CloseButton));
    await tester.pumpAndSettle();
    await scrollAndTap(tester, find.text('Test Shop').last);
    expect(find.byType(ShopFormScreen), findsOneWidget);
  });

  /// 評価の推移の折れ線の点 (x と y の組) を読む。
  List<List<double>> ratingHistoryValues(WidgetTester tester) {
    final chart = tester.widget<LineChart>(
      find.byKey(const Key('purchase-rating-history-chart')),
    );
    return chart.data.lineBarsData.single.spots
        .map((spot) => <double>[spot.x, spot.y])
        .toList();
  }

  testWidgets('詳細でその購入の評価の推移の折れ線グラフを表示する', (tester) async {
    final api = await openPurchaseDetail(
      tester,
      ratings: <Map<String, Object?>>[
        ratingHistoryJson(id: 'brew-1', brewedAt: '2026-09-01T00:00:00.000Z', rating: 2),
        ratingHistoryJson(id: 'brew-2', brewedAt: '2026-09-10T00:00:00.000Z', rating: 5),
      ],
    );

    expect(api.calls, contains('GET /api/purchases/purchase-1/rating-history'));
    // 抽出日時の昇順の抽出の評価を、折れ線の系列にする (FR-18)。
    expect(ratingHistoryValues(tester), <List<double>>[
      <double>[0.0, 2.0],
      <double>[1.0, 5.0],
    ]);
  });

  testWidgets('評価の推移の読み込みが重なっても古い応答で上書きしない', (tester) async {
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
        '/api/purchases/purchase-1',
        status: 200,
        body: purchaseJson(id: 'purchase-1'),
      )
      ..on(
        'POST',
        '/api/purchases/purchase-1/archive',
        status: 200,
        body: purchaseJson(id: 'purchase-1', archivedAt: '2026-09-03T00:00:00.000Z'),
      );
    // 1 回目の評価は 2、2 回目は 5 にして、どちらの応答が表示されたか分かるようにする。
    var ratingsCalls = 0;
    api.onQuery('GET', '/api/purchases/purchase-1/rating-history', (query) {
      ratingsCalls += 1;
      final rating = ratingsCalls == 1 ? 2 : 5;
      return (
        status: 200,
        body: <String, Object?>{
          'ratings': <Map<String, Object?>>[
            ratingHistoryJson(id: 'brew-1', brewedAt: '2026-09-01T00:00:00.000Z', rating: rating),
          ],
        },
      );
    });
    // 最初の読み込みの評価の推移の応答を保留し、その間にアーカイブで読み直させる。
    api.holdOnce('GET', '/api/purchases/purchase-1/rating-history');
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());
    await openLocation(tester, AppRoutes.purchasePath('purchase-1'));

    await tester.tap(find.byTooltip(l10n.archiveButton));
    for (var i = 0; i < 10; i++) {
      await tester.pump(const Duration(milliseconds: 20));
    }
    // 保留されていない 2 回目の読み込みの応答が表示される (応答は消費の順に 2、5 を返す)。
    final shown = ratingHistoryValues(tester);
    expect(shown, <List<double>>[
      <double>[0.0, 2.0],
    ]);

    // 古い読み込みの応答 (5) を解放しても、表示は変わらない。
    api.release('GET', '/api/purchases/purchase-1/rating-history');
    await tester.pumpAndSettle();

    expect(ratingHistoryValues(tester), shown);
  });

  testWidgets('詳細で評価の推移の読み込みに失敗すると再試行を促す', (tester) async {
    final l10n = await loadL10n();
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      )
      ..on('GET', '/api/purchases/purchase-1', status: 200, body: purchaseJson(id: 'purchase-1'))
      ..onNetworkError('GET', '/api/purchases/purchase-1/rating-history');
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());
    await openLocation(tester, AppRoutes.purchasePath('purchase-1'));

    // 購入の表示は残し、グラフの区画にだけ失敗を出す。
    expect(find.text(l10n.errorNetwork), findsOneWidget);
    expect(find.text(l10n.purchaseDetailTitle), findsOneWidget);

    api.on(
      'GET',
      '/api/purchases/purchase-1/rating-history',
      status: 200,
      body: <String, Object?>{
        'ratings': <Object?>[
          ratingHistoryJson(id: 'brew-1', brewedAt: '2026-09-01T00:00:00.000Z', rating: 4),
        ],
      },
    );
    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.retryButton));

    expect(find.text(l10n.errorNetwork), findsNothing);
    expect(find.byKey(const Key('purchase-rating-history-chart')), findsOneWidget);
  });

  testWidgets('詳細でアーカイブとアーカイブ解除ができる', (tester) async {
    final l10n = await loadL10n();
    final api = await openPurchaseDetail(tester);
    api
      ..on(
        'POST',
        '/api/purchases/purchase-1/archive',
        status: 200,
        body: purchaseJson(id: 'purchase-1', archivedAt: '2026-09-03T00:00:00.000Z'),
      )
      // アーカイブの後は、単件の取得もアーカイブ済みを返す (画面は通知で読み直す)。
      ..on(
        'GET',
        '/api/purchases/purchase-1',
        status: 200,
        body: purchaseJson(id: 'purchase-1', archivedAt: '2026-09-03T00:00:00.000Z'),
      )
      ..on(
        'POST',
        '/api/purchases/purchase-1/unarchive',
        status: 200,
        body: purchaseJson(id: 'purchase-1'),
      );

    await tester.tap(find.byTooltip(l10n.archiveButton));
    await tester.pumpAndSettle();

    expect(api.calls, contains('POST /api/purchases/purchase-1/archive'));
    expect(find.text(l10n.archivedMessage), findsOneWidget);
    // 通知が消えるまで進める (次の通知が待ち行列に残らないようにする)。
    await tester.pump(const Duration(seconds: 5));
    await tester.pumpAndSettle();

    // アーカイブ済みの購入は、アーカイブ解除で戻せる (FR-12)。
    await tester.tap(find.byTooltip(l10n.unarchiveButton));
    await tester.pumpAndSettle();

    expect(api.calls, contains('POST /api/purchases/purchase-1/unarchive'));
    expect(find.text(l10n.unarchivedMessage), findsOneWidget);
  });

  testWidgets('購入を登録する (商品と店と購入日の既定値)', (tester) async {
    final l10n = await loadL10n();
    final api = await openPurchaseForm(tester);
    api.on(
      'POST',
      '/api/purchases',
      status: 200,
      body: purchaseJson(id: 'purchase-9', purchasedOn: formatDay(today())),
    );

    // 購入日の既定値は端末のタイムゾーンでの当日 (FR-9)。
    expect(find.widgetWithText(TextField, formatDay(today())), findsOneWidget);

    await pickFromDialog(tester, l10n.productLabel, 'Ethiopia');
    await pickFromDialog(tester, l10n.shopLabel, 'Test Shop');
    await scrollAndEnterText(tester, find.byType(TextField).at(1), 'Medium');
    await scrollAndEnterText(tester, find.byType(TextField).at(3), '1200');
    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.saveButton));

    expect(api.lastBody('POST', '/api/purchases'), <String, Object?>{
      'product_id': 'product-1',
      'shop_id': 'shop-1',
      'purchased_on': formatDay(today()),
      'roast': 'Medium',
      'roast_date': null,
      'price_amount': 1200,
      'price_currency': 'JPY',
      'weight_grams': null,
    });
    expect(find.text(l10n.savedMessage), findsOneWidget);
  });

  testWidgets('商品を選ばないとエラーを表示し、API を呼ばない', (tester) async {
    final l10n = await loadL10n();
    final api = await openPurchaseForm(tester);

    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.saveButton));

    expect(find.text(l10n.validationProduct), findsOneWidget);
    expect(api.calls, <String>['GET /api/passkeys', 'GET /api/brews']);
  });

  testWidgets('価格と重量の検証エラーを表示する', (tester) async {
    final l10n = await loadL10n();
    final api = await openPurchaseForm(tester);

    await pickFromDialog(tester, l10n.productLabel, 'Ethiopia');
    await scrollAndEnterText(tester, find.byType(TextField).at(3), '1.5.2');
    await scrollAndEnterText(tester, find.byType(TextField).at(5), 'abc');
    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.saveButton));

    expect(find.text(l10n.validationNumber), findsNWidgets(2));
    expect(api.calls, <String>['GET /api/passkeys', 'GET /api/brews', 'GET /api/products']);

    // 通貨コードは ISO 4217 の 3 文字の英大文字だけを受け付ける (FR-9。入力は大文字に直してから検証する)。
    await scrollAndEnterText(tester, find.byType(TextField).at(3), '1200');
    await scrollAndEnterText(tester, find.byType(TextField).at(4), 'us');
    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.saveButton));

    expect(find.text(l10n.validationCurrency), findsOneWidget);

    // 小文字の 3 文字は大文字に直してから検証するため通る (FR-9)。
    api.on('POST', '/api/purchases', status: 200, body: purchaseJson(id: 'purchase-1'));
    await scrollAndEnterText(tester, find.byType(TextField).at(4), 'usd');
    await scrollAndEnterText(tester, find.byType(TextField).at(5), '100');
    await pickFromDialog(tester, l10n.productLabel, 'Ethiopia');
    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.saveButton));

    expect(find.text(l10n.validationCurrency), findsNothing);
    expect(api.lastBody('POST', '/api/purchases')?['price_currency'], 'USD');
  });

  testWidgets('購入を編集し、店を外せる', (tester) async {
    final l10n = await loadL10n();
    final api = await openPurchaseForm(tester, id: 'purchase-1');
    api.on('PATCH', '/api/purchases/purchase-1', status: 200, body: purchaseJson());
    // 編集では現在の値を読む (FR-9)。
    api.on(
      'GET',
      '/api/purchases/purchase-1',
      status: 200,
      body: purchaseJson(id: 'purchase-1', purchasedOn: '2026-08-01', shop: shopJson()),
    );
    await tester.pumpAndSettle();

    await scrollAndTap(tester, find.text(l10n.shopLabel));
    await tester.pumpAndSettle();
    // 店を指定しない選択肢で参照を外す (FR-9)。
    // ピッカーの未選択の表示にも同じ文言が出るため、ダイアログの選択肢を選ぶ。
    await tester.tap(find.text(l10n.shopNoneLabel).last);
    await tester.pumpAndSettle();
    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.saveButton));

    expect(api.lastBody('PATCH', '/api/purchases/purchase-1'), <String, Object?>{
      'product_id': 'product-1',
      'shop_id': null,
      'purchased_on': '2026-08-01',
      'roast': null,
      'roast_date': null,
      'price_amount': null,
      'price_currency': null,
      'weight_grams': null,
    });
  });

  testWidgets('写真を選び、変換したサイズを申告して URL の発行と PUT と完了通知を行う', (tester) async {
    final l10n = await loadL10n();
    final picker = FakePhotoPicker(
      photo: PickedPhoto(
        name: 'package.png',
        bytes: Uint8List.fromList(List<int>.filled(4096, 1)),
      ),
    );
    // 変換の結果は 1234 バイトの JPEG とする (申告サイズの確認に使う)。
    final converter = FakeImageConverter(
      bytes: Uint8List.fromList(<int>[0xFF, 0xD8, 0xFF, ...List<int>.filled(1231, 2)]),
    );
    final upload = FakeUploadClient();
    final api = await openPurchaseForm(
      tester,
      picker: picker,
      converter: converter,
      upload: upload,
    );
    api
      ..on('POST', '/api/purchases', status: 200, body: purchaseJson(id: 'purchase-9'))
      ..on(
        'POST',
        '/api/purchase-suggestions',
        status: 200,
        body: emptySuggestionJson(),
      )
      ..on(
        'POST',
        '/api/purchases/purchase-9/photo/upload-url',
        status: 200,
        body: <String, Object?>{'url': 'https://r2.example/upload', 'key': 'pending/user-1/x.jpg'},
      )
      ..on(
        'POST',
        '/api/purchases/purchase-9/photo',
        status: 200,
        body: purchaseJson(
          id: 'purchase-9',
          photoKey: 'users/user-1/purchases/purchase-9/x.jpg',
        ),
      );

    // 写真を選び、クライアントで JPEG へ変換する (FR-10)。
    await scrollAndTap(tester, find.widgetWithText(OutlinedButton, l10n.photoSelectButton));
    expect(picker.calls, 1);
    expect(converter.calls, 1);
    expect(converter.lastInputSize, 4096);
    expect(find.byKey(const Key('purchase-photo')), findsOneWidget);

    await pickFromDialog(tester, l10n.productLabel, 'Ethiopia');
    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.saveButton));

    // 変換後のサイズを申告して URL を要求する (ADR-0003)。
    expect(api.lastBody('POST', '/api/purchases/purchase-9/photo/upload-url'), <String, Object?>{
      'size': 1234,
    });
    // R2 へ直接 PUT する (Content-Type は image/jpeg)。
    final request = upload.requests.single;
    expect(request.url.toString(), 'https://r2.example/upload');
    expect(request.headers['Content-Type'], 'image/jpeg');
    expect(request.bodyBytes.length, 1234);
    // 完了を通知する (ADR-0003)。
    expect(api.lastBody('POST', '/api/purchases/purchase-9/photo'), <String, Object?>{
      'key': 'pending/user-1/x.jpg',
      'size': 1234,
    });
    expect(api.calls, <String>[
      'GET /api/passkeys',
      'GET /api/brews',
      // 写真を選んだ直後に推測を呼ぶ (FR-19)。
      'POST /api/purchase-suggestions',
      'GET /api/products',
      'POST /api/purchases',
      'POST /api/purchases/purchase-9/photo/upload-url',
      'POST /api/purchases/purchase-9/photo',
      // 保存の通知でホームの一覧も読み直す (ホームは下に残っている)。
      'GET /api/brews',
    ]);
    expect(find.text(l10n.savedMessage), findsOneWidget);
  });

  testWidgets('写真の差し替えは購入の更新の後にアップロードする', (tester) async {
    final l10n = await loadL10n();
    final picker = FakePhotoPicker(
      photo: PickedPhoto(name: 'new.png', bytes: Uint8List.fromList(<int>[1, 2, 3])),
    );
    final converter = FakeImageConverter(bytes: Uint8List.fromList(<int>[0xFF, 0xD8, 0xFF]));
    final upload = FakeUploadClient();
    final api = await openPurchaseForm(
      tester,
      id: 'purchase-1',
      purchase: purchaseJson(
        id: 'purchase-1',
        photoKey: 'users/user-1/purchases/purchase-1/old.jpg',
      ),
      picker: picker,
      converter: converter,
      upload: upload,
    );
    api
      ..on('PATCH', '/api/purchases/purchase-1', status: 200, body: purchaseJson())
      ..on(
        'POST',
        '/api/purchase-suggestions',
        status: 200,
        body: emptySuggestionJson(),
      )
      ..on(
        'POST',
        '/api/purchases/purchase-1/photo/upload-url',
        status: 200,
        body: <String, Object?>{'url': 'https://r2.example/upload', 'key': 'pending/user-1/y.jpg'},
      )
      ..on(
        'POST',
        '/api/purchases/purchase-1/photo',
        status: 200,
        body: purchaseJson(id: 'purchase-1', photoKey: 'users/user-1/purchases/purchase-1/y.jpg'),
      );
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('purchase-photo')), findsOneWidget);
    await scrollAndTap(tester, find.widgetWithText(OutlinedButton, l10n.photoReplaceButton));
    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.saveButton));

    expect(api.lastBody('POST', '/api/purchases/purchase-1/photo'), <String, Object?>{
      'key': 'pending/user-1/y.jpg',
      'size': 3,
    });
    // 写真の差し替えは購入の更新の後にアップロードする (PUT の前の upload-url の要求で確かめる)。
    expect(api.calls, contains('PATCH /api/purchases/purchase-1'));
    expect(
      api.calls.indexOf('PATCH /api/purchases/purchase-1'),
      lessThan(api.calls.indexOf('POST /api/purchases/purchase-1/photo/upload-url')),
    );
  });

  testWidgets('写真を削除して保存すると購入から外れる', (tester) async {
    final l10n = await loadL10n();
    final upload = FakeUploadClient();
    final api = await openPurchaseForm(
      tester,
      id: 'purchase-1',
      purchase: purchaseJson(
        id: 'purchase-1',
        photoKey: 'users/user-1/purchases/purchase-1/old.jpg',
      ),
      upload: upload,
    );
    api
      ..on('PATCH', '/api/purchases/purchase-1', status: 200, body: purchaseJson())
      ..on(
        'DELETE',
        '/api/purchases/purchase-1/photo',
        status: 200,
        body: purchaseJson(id: 'purchase-1'),
      );
    await tester.pumpAndSettle();

    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.photoDeleteButton));
    expect(find.widgetWithText(OutlinedButton, l10n.photoSelectButton), findsOneWidget);
    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.saveButton));

    expect(api.calls, contains('DELETE /api/purchases/purchase-1/photo'));
    expect(api.calls, contains('PATCH /api/purchases/purchase-1'));
  });

  testWidgets('写真の削除の後に保存が失敗しても、やり直しで保存できる', (tester) async {
    final l10n = await loadL10n();
    final api = await openPurchaseForm(
      tester,
      id: 'purchase-1',
      purchase: purchaseJson(
        id: 'purchase-1',
        photoKey: 'users/user-1/purchases/purchase-1/old.jpg',
      ),
    );
    api
      ..on(
        'DELETE',
        '/api/purchases/purchase-1/photo',
        status: 200,
        body: purchaseJson(id: 'purchase-1'),
      )
      ..onNetworkError('PATCH', '/api/purchases/purchase-1');
    await tester.pumpAndSettle();

    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.photoDeleteButton));
    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.saveButton));
    // 1 回目の保存は購入の更新の失敗で終わる (削除は成功している)。
    expect(find.text(l10n.errorNetwork), findsOneWidget);

    // やり直しでは削除を繰り返さず、更新だけを行う。
    api.on(
      'PATCH',
      '/api/purchases/purchase-1',
      status: 200,
      body: purchaseJson(id: 'purchase-1'),
    );
    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.saveButton));
    await tester.pumpAndSettle();

    expect(
      api.calls.where((call) => call == 'DELETE /api/purchases/purchase-1/photo').length,
      1,
    );
    expect(find.text(l10n.savedMessage), findsOneWidget);
  });

  testWidgets('範囲外の日付でもカレンダーを開ける', (tester) async {
    await loadL10n();
    await openPurchaseForm(tester);
    // `l10n.selectButton` の tooltip は参照の選択にも付くため、日付の欄のカレンダーのアイコンを狙う。
    final calendar = find.byIcon(Icons.calendar_today_outlined).first;
    for (final date in ['1999-12-31', '2101-01-01']) {
      // 検証が許す 4 桁の年でも、カレンダーの初期値を範囲 (2000-2100) に収めて開ける
      // (範囲外だと debug の assert で落ちる)。
      await scrollAndEnterText(tester, find.byType(TextField).at(0), date);
      await scrollAndTap(tester, calendar);
      await tester.pumpAndSettle();
      expect(find.byType(Dialog), findsOneWidget, reason: date);
      // 閉じる (次の入力の前にダイアログを消す)。
      await tester.tap(find.text('Cancel').first);
      await tester.pumpAndSettle();
    }
  });

  testWidgets('写真のアップロードが接続の失敗なら再試行を促す', (tester) async {
    final l10n = await loadL10n();
    final picker = FakePhotoPicker(
      photo: PickedPhoto(name: 'package.png', bytes: Uint8List.fromList(<int>[1, 2, 3])),
    );
    final upload = FakeUploadClient()..networkError = true;
    final api = await openPurchaseForm(
      tester,
      picker: picker,
      converter: FakeImageConverter(),
      upload: upload,
    );
    api
      ..on(
        'POST',
        '/api/purchase-suggestions',
        status: 200,
        body: emptySuggestionJson(),
      )
      ..on(
        'POST',
        '/api/purchases',
        status: 200,
        body: purchaseJson(id: 'purchase-1'),
      )
      ..on(
        'POST',
        '/api/purchases/purchase-1/photo/upload-url',
        status: 200,
        body: <String, Object?>{'url': 'https://r2.example/upload', 'key': 'pending/user-1/z.jpg'},
      );

    await pickFromDialog(tester, l10n.productLabel, 'Ethiopia');
    await scrollAndTap(tester, find.widgetWithText(OutlinedButton, l10n.photoSelectButton));
    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.saveButton));

    expect(find.text(l10n.errorNetwork), findsOneWidget);
  });

  testWidgets('写真のアップロードに失敗すると再試行を促す', (tester) async {
    final l10n = await loadL10n();
    final picker = FakePhotoPicker(
      photo: PickedPhoto(name: 'package.png', bytes: Uint8List.fromList(<int>[1, 2, 3])),
    );
    final upload = FakeUploadClient()..status = 403;
    final api = await openPurchaseForm(
      tester,
      picker: picker,
      converter: FakeImageConverter(),
      upload: upload,
    );
    api
      ..on(
        'POST',
        '/api/purchase-suggestions',
        status: 200,
        body: emptySuggestionJson(),
      )
      ..on('POST', '/api/purchases', status: 200, body: purchaseJson(id: 'purchase-9'))
      ..on(
        'POST',
        '/api/purchases/purchase-9/photo/upload-url',
        status: 200,
        body: <String, Object?>{'url': 'https://r2.example/upload', 'key': 'pending/user-1/x.jpg'},
      );

    await scrollAndTap(tester, find.widgetWithText(OutlinedButton, l10n.photoSelectButton));
    await pickFromDialog(tester, l10n.productLabel, 'Ethiopia');
    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.saveButton));

    // R2 の PUT の失敗は、再試行を促す表示にする (ADR-0007)。
    expect(find.text(l10n.errorNetwork), findsOneWidget);
    expect(find.byType(PurchaseFormScreen), findsOneWidget);

    // 再試行では、登録済みの購入を更新して写真をアップロードし直す (購入を 2 つ作らない)。
    upload.status = 200;
    api.on(
      'PATCH',
      '/api/purchases/purchase-9',
      status: 200,
      body: purchaseJson(id: 'purchase-9'),
    );
    api.on(
      'POST',
      '/api/purchases/purchase-9/photo',
      status: 200,
      body: purchaseJson(id: 'purchase-9', photoKey: 'users/user-1/purchases/purchase-9/x.jpg'),
    );
    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.saveButton));

    expect(api.calls.where((call) => call == 'POST /api/purchases').length, 1);
    expect(api.calls, contains('PATCH /api/purchases/purchase-9'));
    expect(find.text(l10n.savedMessage), findsOneWidget);
  });

  testWidgets('焙煎度で候補を表示し、候補に無い値も入力できる', (tester) async {
    final api = await openPurchaseForm(tester);
    // 焙煎度は自由記述で、入力中に過去の入力値の候補を出す (FR-13)。
    final roast = find.byType(TextField).at(1);
    await scrollAndEnterText(tester, roast, 'Medi');
    await tester.pump(const Duration(milliseconds: 400));
    await tester.pumpAndSettle();
    expect(find.text('Medium'), findsOneWidget);
    expect(api.calls, contains('GET /api/suggestions/roast'));

    // 候補に無い値もそのまま入力できる (FR-13)。
    await scrollAndEnterText(tester, roast, 'Custom Roast');
    expect(find.widgetWithText(TextField, 'Custom Roast'), findsOneWidget);
  });

  testWidgets('入力の検証エラー (400) を表示する', (tester) async {
    final l10n = await loadL10n();
    final api = await openPurchaseForm(tester);
    api.on('POST', '/api/purchases', status: 400, body: badRequestBody);

    await pickFromDialog(tester, l10n.productLabel, 'Ethiopia');
    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.saveButton));

    expect(find.text(l10n.errorValidation), findsOneWidget);
    expect(find.byType(PurchaseFormScreen), findsOneWidget);
  });

  testWidgets('認証の失効 (401) ではログイン画面へ遷移する', (tester) async {
    final l10n = await loadL10n();
    final api = await openPurchaseForm(tester);
    api.on('POST', '/api/purchases', status: 401, body: unauthorizedBody);

    await pickFromDialog(tester, l10n.productLabel, 'Ethiopia');
    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.saveButton));

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
      ..onNetworkError('GET', '/api/purchases/purchase-1');
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());
    await openLocation(tester, AppRoutes.purchaseEditPath('purchase-1'));

    expect(find.text(l10n.errorNetwork), findsOneWidget);

    // 再試行で読み込むと、失敗の表示が消えて値が入る。
    api.on(
      'GET',
      '/api/purchases/purchase-1',
      status: 200,
      body: purchaseJson(id: 'purchase-1', roast: 'Retried Roast'),
    );
    await tester.tap(find.widgetWithText(TextButton, l10n.retryButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.errorNetwork), findsNothing);
    expect(find.widgetWithText(TextField, 'Retried Roast'), findsOneWidget);
  });

  // 写真からの推測の反映 (FR-19、ADR-0016)。

  /// 推測の応答を組み立てる。無い項目は null にする。
  Map<String, Object?> suggestionJson({
    Map<String, Object?>? product,
    String? roast,
    String? roastDate,
    int? priceAmount,
    int? weightGrams,
  }) {
    return <String, Object?>{
      'product': product,
      'roast': roast,
      'roast_date': roastDate,
      'price_amount': priceAmount,
      'weight_grams': weightGrams,
    };
  }

  /// 商品の一覧の応答を、名前の完全一致の絞り込み (FR-19) に応じて変える。
  ///
  /// 絞り込みが無い呼び出し (商品の選択のダイアログ) は既定の 1 件を返す。
  void onProductNameLookup(FakeApi api, List<Map<String, Object?>> matched) {
    api.onQuery('GET', '/api/products', (query) {
      if (query['name'] == null) {
        return (
          status: 200,
          body: pageJson(
            key: 'products',
            items: <Map<String, Object?>>[productJson(id: 'product-1', name: 'Ethiopia')],
          ),
        );
      }
      return (status: 200, body: pageJson(key: 'products', items: matched));
    });
  }

  /// 推測の商品の項目を組み立てる。
  Map<String, Object?> suggestedProductJson({
    String? name = 'New Bean',
    String? producer,
    String? origin,
    String? region,
    String? process,
    String? variety,
    List<String> flavorNotes = const <String>[],
  }) {
    return <String, Object?>{
      'name': name,
      'producer': producer,
      'origin': origin,
      'region': region,
      'process': process,
      'variety': variety,
      'flavor_notes': flavorNotes,
    };
  }

  testWidgets('写真を選ぶと推測を呼び、空の入力欄にだけ反映する', (tester) async {
    final l10n = await loadL10n();
    final picker = FakePhotoPicker(
      photo: PickedPhoto(name: 'package.png', bytes: Uint8List.fromList(<int>[1, 2, 3])),
    );
    final converter = FakeImageConverter(
      bytes: Uint8List.fromList(<int>[0xFF, 0xD8, 0xFF, 4, 5]),
    );
    final api = await openPurchaseForm(tester, picker: picker, converter: converter);
    onProductNameLookup(api, <Map<String, Object?>>[
      productJson(id: 'product-2', name: 'Ethiopia Guji'),
    ]);
    api.on(
      'POST',
      '/api/purchase-suggestions',
      status: 200,
      body: suggestionJson(
        product: suggestedProductJson(
          name: 'Ethiopia Guji',
          producer: 'Guji',
          origin: 'Ethiopia',
          process: 'Washed',
          flavorNotes: <String>['Floral'],
        ),
        roast: 'Medium',
        roastDate: '2026-09-20',
        priceAmount: 1200,
        weightGrams: 200,
      ),
    );

    // 入力済みの焙煎度と価格は上書きしない (FR-19)。
    await scrollAndEnterText(tester, find.byType(TextField).at(1), 'Custom Roast');
    await scrollAndEnterText(tester, find.byType(TextField).at(3), '999');
    await scrollAndTap(tester, find.widgetWithText(OutlinedButton, l10n.photoSelectButton));

    // 変換済みの JPEG をそのまま送る (FR-19)。
    final request = api.requests.lastWhere(
      (request) => request.url.path == '/api/purchase-suggestions',
    );
    expect(request.headers['Content-Type'], 'image/jpeg');
    expect(request.bodyBytes, <int>[0xFF, 0xD8, 0xFF, 4, 5]);

    // 空の入力欄だけに反映する (入力済みの焙煎度と価格は残る)。
    expect(find.widgetWithText(TextField, 'Custom Roast'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Medium'), findsNothing);
    expect(find.widgetWithText(TextField, '999'), findsOneWidget);
    expect(find.widgetWithText(TextField, '1200'), findsNothing);
    expect(find.widgetWithText(TextField, '2026-09-20'), findsOneWidget);
    expect(find.widgetWithText(TextField, '200'), findsOneWidget);
    // 通貨は推測せず、既定値の JPY のままにする (FR-19)。
    expect(find.widgetWithText(TextField, 'JPY'), findsOneWidget);
    // 商品が未選択のときだけ、名前の完全一致 (前後の空白を除き、大文字と小文字を区別しない) で
    // 一致する商品を選ぶ (FR-19)。
    expect(api.lastQuery('GET', '/api/products')?['name'], 'Ethiopia Guji');
    expect(find.text('Ethiopia Guji'), findsOneWidget);
    // 推測はどの記録も変更しない (FR-19)。購入の API は呼ばれない。
    expect(api.calls.where((call) => call.contains('/api/purchases')), isEmpty);
  });

  testWidgets('推測中はインジケータを出し、失敗するとバナーを出して手入力を続けられる', (tester) async {
    final l10n = await loadL10n();
    final picker = FakePhotoPicker(
      photo: PickedPhoto(name: 'package.png', bytes: Uint8List.fromList(<int>[1, 2, 3])),
    );
    final api = await openPurchaseForm(
      tester,
      picker: picker,
      converter: FakeImageConverter(bytes: Uint8List.fromList(<int>[0xFF, 0xD8, 0xFF])),
      upload: FakeUploadClient(),
    );
    // AI の呼び出しの失敗 (500) を返す (FR-19)。
    api
      ..on(
        'POST',
        '/api/purchase-suggestions',
        status: 500,
        body: <String, Object?>{
          'error': <String, Object?>{
            'code': 'internal_error',
            'message': 'the suggestion failed',
          },
        },
      )
      // 応答を保留して、推測中の表示を確認できるようにする。
      ..holdOnce('POST', '/api/purchase-suggestions');

    final select = find.widgetWithText(OutlinedButton, l10n.photoSelectButton);
    await tester.ensureVisible(select);
    await tester.pumpAndSettle();
    await tester.tap(select);
    for (var i = 0; i < 5; i++) {
      await tester.pump(const Duration(milliseconds: 10));
    }
    // 推測中はインジケータを出す (FR-19)。
    expect(find.text(l10n.suggestionLoadingLabel), findsOneWidget);

    api.release('POST', '/api/purchase-suggestions');
    await tester.pumpAndSettle();

    // 失敗はバナーで表示し、写真の選択は残る (FR-19)。
    expect(find.text(l10n.suggestionFailedMessage), findsOneWidget);
    expect(find.byKey(const Key('purchase-photo')), findsOneWidget);

    // 手入力を続けられる (商品を選んで保存できる)。
    api
      ..on('POST', '/api/purchases', status: 200, body: purchaseJson(id: 'purchase-1'))
      ..on(
        'POST',
        '/api/purchases/purchase-1/photo/upload-url',
        status: 200,
        body: <String, Object?>{'url': 'https://r2.example/upload', 'key': 'pending/user-1/a.jpg'},
      )
      ..on(
        'POST',
        '/api/purchases/purchase-1/photo',
        status: 200,
        body: purchaseJson(
          id: 'purchase-1',
          photoKey: 'users/user-1/purchases/purchase-1/a.jpg',
        ),
      );
    await pickFromDialog(tester, l10n.productLabel, 'Ethiopia');
    await scrollAndEnterText(tester, find.byType(TextField).at(1), 'Manual Roast');
    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.saveButton));

    expect(api.calls, contains('POST /api/purchases'));
    expect(find.text(l10n.savedMessage), findsOneWidget);
  });

  testWidgets('一致する商品が無いときは推測した内容で商品を登録し、その商品を選択する', (tester) async {
    final l10n = await loadL10n();
    final picker = FakePhotoPicker(
      photo: PickedPhoto(name: 'package.png', bytes: Uint8List.fromList(<int>[1, 2, 3])),
    );
    final api = await openPurchaseForm(
      tester,
      picker: picker,
      converter: FakeImageConverter(bytes: Uint8List.fromList(<int>[0xFF, 0xD8, 0xFF])),
    );
    // 名前の完全一致に一致する商品は無い。
    onProductNameLookup(api, <Map<String, Object?>>[]);
    api
      ..on(
        'POST',
        '/api/purchase-suggestions',
        status: 200,
        body: suggestionJson(
          product: suggestedProductJson(
            producer: 'New Producer',
            origin: 'Ethiopia',
            region: 'Guji',
            process: 'Washed',
            variety: 'Heirloom',
            flavorNotes: <String>['Floral', 'Citrus'],
          ),
          roast: 'Medium',
        ),
      )
      ..on(
        'POST',
        '/api/products',
        status: 200,
        body: productJson(id: 'product-9', name: 'New Bean'),
      );

    await scrollAndTap(tester, find.widgetWithText(OutlinedButton, l10n.photoSelectButton));

    // 一致が無いときは、推測した内容で商品を登録する導線を出す (FR-19)。
    await scrollAndTap(
      tester,
      find.widgetWithText(OutlinedButton, l10n.suggestionRegisterProductButton),
    );

    // 商品のフォームに推測値 (商品名、Producer、Origin、Region、Process、Variety、
    // Flavor Notes) を引き継ぐ (FR-19)。
    expect(find.byType(ProductFormScreen), findsOneWidget);
    expect(find.widgetWithText(TextField, 'New Bean'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'New Producer'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Ethiopia'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Guji'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Washed'), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Heirloom'), findsOneWidget);
    expect(find.text('Floral'), findsOneWidget);
    expect(find.text('Citrus'), findsOneWidget);

    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.saveButton));

    // 登録した商品の本文に推測値が入る。
    expect(api.lastBody('POST', '/api/products'), <String, Object?>{
      'name': 'New Bean',
      'producer': 'New Producer',
      'origin': 'Ethiopia',
      'region': 'Guji',
      'process': 'Washed',
      'variety': 'Heirloom',
      'flavor_notes': <String>['Floral', 'Citrus'],
    });
    // 購入のフォームに戻り、登録した商品を選択する (FR-19)。
    expect(find.byType(PurchaseFormScreen), findsOneWidget);
    expect(find.text('New Bean'), findsOneWidget);
    expect(
      find.widgetWithText(OutlinedButton, l10n.suggestionRegisterProductButton),
      findsNothing,
    );
    // 推測は保存を伴わない (購入の API はまだ呼ばれない)。
    expect(api.calls.where((call) => call.contains('/api/purchases')), isEmpty);
  });

  testWidgets('選択済みの商品は推測で上書きしない', (tester) async {
    final l10n = await loadL10n();
    final picker = FakePhotoPicker(
      photo: PickedPhoto(name: 'package.png', bytes: Uint8List.fromList(<int>[1, 2, 3])),
    );
    final api = await openPurchaseForm(
      tester,
      picker: picker,
      converter: FakeImageConverter(bytes: Uint8List.fromList(<int>[0xFF, 0xD8, 0xFF])),
    );
    onProductNameLookup(api, <Map<String, Object?>>[
      productJson(id: 'product-2', name: 'Ethiopia Guji'),
    ]);
    api.on(
      'POST',
      '/api/purchase-suggestions',
      status: 200,
      body: suggestionJson(
        product: suggestedProductJson(name: 'Ethiopia Guji'),
        roast: 'Medium',
      ),
    );

    // 先に商品を選んでおく (FR-19)。
    await pickFromDialog(tester, l10n.productLabel, 'Ethiopia');
    await scrollAndTap(tester, find.widgetWithText(OutlinedButton, l10n.photoSelectButton));

    // 選択済みの商品は上書きせず、一致の探索もしない。
    expect(find.text('Ethiopia'), findsOneWidget);
    expect(find.text('Ethiopia Guji'), findsNothing);
    expect(
      api.calls.where((call) => call == 'GET /api/products').length,
      1,
      reason: 'the product picker must be the only products call',
    );
    // 一致が無いときの導線も出さない。
    expect(
      find.widgetWithText(OutlinedButton, l10n.suggestionRegisterProductButton),
      findsNothing,
    );
    // 空の焙煎度には反映する。
    expect(find.widgetWithText(TextField, 'Medium'), findsOneWidget);
  });

  testWidgets('推測の待ち中に選んだ商品は上書きしない', (tester) async {
    final l10n = await loadL10n();
    final picker = FakePhotoPicker(
      photo: PickedPhoto(name: 'package.png', bytes: Uint8List.fromList(<int>[1, 2, 3])),
    );
    final api = await openPurchaseForm(
      tester,
      picker: picker,
      converter: FakeImageConverter(bytes: Uint8List.fromList(<int>[0xFF, 0xD8, 0xFF])),
    );
    onProductNameLookup(api, <Map<String, Object?>>[
      productJson(id: 'product-2', name: 'Ethiopia Guji'),
    ]);
    api
      ..on(
        'POST',
        '/api/purchase-suggestions',
        status: 200,
        body: suggestionJson(
          product: suggestedProductJson(name: 'Ethiopia Guji'),
          roast: 'Medium',
        ),
      )
      // 推測の応答を保留して、待ち中の選択を再現する (FR-19)。
      ..holdOnce('POST', '/api/purchase-suggestions');

    // 推測中はインジケータが動くため、`pumpAndSettle` は使わずに手動で進める。
    final select = find.widgetWithText(OutlinedButton, l10n.photoSelectButton);
    await tester.ensureVisible(select);
    await tester.pumpAndSettle();
    await tester.tap(select);
    for (var i = 0; i < 5; i++) {
      await tester.pump(const Duration(milliseconds: 10));
    }

    // 推測の待ち中に商品を選ぶ (FR-19)。
    await tester.ensureVisible(find.text(l10n.productLabel));
    await tester.pump();
    await tester.tap(find.text(l10n.productLabel));
    for (var i = 0; i < 10; i++) {
      await tester.pump(const Duration(milliseconds: 10));
    }
    await tester.tap(find.text('Ethiopia').last);
    for (var i = 0; i < 10; i++) {
      await tester.pump(const Duration(milliseconds: 10));
    }

    api.release('POST', '/api/purchase-suggestions');
    await tester.pumpAndSettle();
    // 待ち中に選んだ商品を、推測の一致で上書きしない (FR-19)。
    expect(find.text('Ethiopia'), findsOneWidget);
    expect(find.text('Ethiopia Guji'), findsNothing);
    // 商品が選択済みのため、一致の探索もしない (商品の選択のダイアログの 1 回だけ)。
    expect(
      api.calls.where((call) => call == 'GET /api/products').length,
      1,
      reason: 'the product picker must be the only products call',
    );
    // 空の焙煎度には反映する。
    expect(find.widgetWithText(TextField, 'Medium'), findsOneWidget);
  });

  testWidgets('商品の照合が失敗しても、推測の反映は残る', (tester) async {
    final l10n = await loadL10n();
    final picker = FakePhotoPicker(
      photo: PickedPhoto(name: 'package.png', bytes: Uint8List.fromList(<int>[1, 2, 3])),
    );
    final api = await openPurchaseForm(
      tester,
      picker: picker,
      converter: FakeImageConverter(bytes: Uint8List.fromList(<int>[0xFF, 0xD8, 0xFF])),
    );
    api
      ..on(
        'POST',
        '/api/purchase-suggestions',
        status: 200,
        body: suggestionJson(
          product: suggestedProductJson(name: 'Ethiopia Guji'),
          roast: 'Medium',
          roastDate: '2026-09-20',
          priceAmount: 1200,
          weightGrams: 200,
        ),
      )
      // 商品の照合 (名前の絞り込み) だけが失敗する (FR-19)。
      ..on(
        'GET',
        '/api/products',
        status: 500,
        body: <String, Object?>{
          'error': <String, Object?>{
            'code': 'internal_error',
            'message': 'the products failed',
          },
        },
      );

    await scrollAndTap(tester, find.widgetWithText(OutlinedButton, l10n.photoSelectButton));

    // 照合の失敗は推測の反映を妨げない (FR-19)。
    expect(find.widgetWithText(TextField, 'Medium'), findsOneWidget);
    expect(find.widgetWithText(TextField, '2026-09-20'), findsOneWidget);
    expect(find.widgetWithText(TextField, '1200'), findsOneWidget);
    expect(find.widgetWithText(TextField, '200'), findsOneWidget);
    // 照合の失敗を推測の失敗として表示しない (FR-19)。
    expect(find.text(l10n.suggestionFailedMessage), findsNothing);
    // 一致の有無が分からないため、商品の登録の導線は出さない (FR-19)。
    expect(
      find.widgetWithText(OutlinedButton, l10n.suggestionRegisterProductButton),
      findsNothing,
    );
  });

  testWidgets('推測の商品の照合中に選んだ商品は上書きしない', (tester) async {
    final l10n = await loadL10n();
    final picker = FakePhotoPicker(
      photo: PickedPhoto(name: 'package.png', bytes: Uint8List.fromList(<int>[1, 2, 3])),
    );
    final api = await openPurchaseForm(
      tester,
      picker: picker,
      converter: FakeImageConverter(bytes: Uint8List.fromList(<int>[0xFF, 0xD8, 0xFF])),
    );
    onProductNameLookup(api, <Map<String, Object?>>[
      productJson(id: 'product-2', name: 'Ethiopia Guji'),
    ]);
    api
      ..on(
        'POST',
        '/api/purchase-suggestions',
        status: 200,
        body: suggestionJson(
          product: suggestedProductJson(name: 'Ethiopia Guji'),
          roast: 'Medium',
        ),
      )
      // 名前の照合 (GET /api/products) の応答を保留して、照合中の選択を再現する (FR-19)。
      ..holdOnce('GET', '/api/products');

    // 推測中はインジケータが動くため、`pumpAndSettle` は使わずに手動で進める。
    final select = find.widgetWithText(OutlinedButton, l10n.photoSelectButton);
    await tester.ensureVisible(select);
    await tester.pumpAndSettle();
    await tester.tap(select);
    for (var i = 0; i < 5; i++) {
      await tester.pump(const Duration(milliseconds: 10));
    }

    // 名前の照合の待ち中に、商品の選択のダイアログから商品を選ぶ (FR-19)。
    await tester.ensureVisible(find.text(l10n.productLabel));
    await tester.pump();
    await tester.tap(find.text(l10n.productLabel));
    for (var i = 0; i < 10; i++) {
      await tester.pump(const Duration(milliseconds: 10));
    }
    await tester.tap(find.text('Ethiopia').last);
    for (var i = 0; i < 10; i++) {
      await tester.pump(const Duration(milliseconds: 10));
    }

    api.release('GET', '/api/products');
    await tester.pumpAndSettle();

    // 照合の待ち中に選んだ商品を、照合の一致で上書きしない (FR-19)。
    expect(find.text('Ethiopia'), findsOneWidget);
    expect(find.text('Ethiopia Guji'), findsNothing);
    // 空の焙煎度には反映する。
    expect(find.widgetWithText(TextField, 'Medium'), findsOneWidget);
  });

  testWidgets('5 MB を超える写真を選び直したら、前の写真の推測を反映しない', (tester) async {
    final l10n = await loadL10n();
    final picker = FakePhotoPicker(
      photo: PickedPhoto(name: 'package.png', bytes: Uint8List.fromList(<int>[1, 2, 3])),
    );
    final converter = FakeImageConverter(bytes: Uint8List.fromList(<int>[0xFF, 0xD8, 0xFF]));
    final api = await openPurchaseForm(tester, picker: picker, converter: converter);
    api
      ..on(
        'POST',
        '/api/purchase-suggestions',
        status: 200,
        body: suggestionJson(roast: 'Medium'),
      )
      // 1 枚目の推測の応答を保留する (FR-19)。
      ..holdOnce('POST', '/api/purchase-suggestions');

    // 1 枚目の写真を選ぶ (推測は保留される)。
    final select = find.widgetWithText(OutlinedButton, l10n.photoSelectButton);
    await tester.ensureVisible(select);
    await tester.pumpAndSettle();
    await tester.tap(select);
    for (var i = 0; i < 5; i++) {
      await tester.pump(const Duration(milliseconds: 10));
    }

    // 5 MB を超える写真を選び直す (推測は呼ばず、前の応答を捨てる。FR-19)。
    converter.bytes = Uint8List(maxPhotoBytes + 1);
    final replace = find.widgetWithText(OutlinedButton, l10n.photoReplaceButton);
    await tester.ensureVisible(replace);
    await tester.pump();
    await tester.tap(replace);
    for (var i = 0; i < 5; i++) {
      await tester.pump(const Duration(milliseconds: 10));
    }
    api.release('POST', '/api/purchase-suggestions');
    await tester.pumpAndSettle();

    // 前の写真の推測を反映しない (FR-19)。
    expect(find.widgetWithText(TextField, 'Medium'), findsNothing);
    // 2 枚目の写真は選択されたまま (アップロードは保存のときに行う)。
    expect(find.byKey(const Key('purchase-photo')), findsOneWidget);
    // 推測の API は 1 枚目で 1 回だけ呼ばれる (2 枚目では呼ばない)。
    expect(
      api.calls.where((call) => call == 'POST /api/purchase-suggestions').length,
      1,
    );
  });

  testWidgets('写真を削除したら、飛んでいる推測を反映しない', (tester) async {
    final l10n = await loadL10n();
    final picker = FakePhotoPicker(
      photo: PickedPhoto(name: 'package.png', bytes: Uint8List.fromList(<int>[1, 2, 3])),
    );
    final api = await openPurchaseForm(
      tester,
      picker: picker,
      converter: FakeImageConverter(bytes: Uint8List.fromList(<int>[0xFF, 0xD8, 0xFF])),
    );
    api
      ..on(
        'POST',
        '/api/purchase-suggestions',
        status: 200,
        body: suggestionJson(roast: 'Medium'),
      )
      // 推測の応答を保留する (FR-19)。
      ..holdOnce('POST', '/api/purchase-suggestions');

    // 写真を選ぶ (推測は保留される)。
    final select = find.widgetWithText(OutlinedButton, l10n.photoSelectButton);
    await tester.ensureVisible(select);
    await tester.pumpAndSettle();
    await tester.tap(select);
    for (var i = 0; i < 5; i++) {
      await tester.pump(const Duration(milliseconds: 10));
    }

    // 推測の待ち中に写真を削除する (FR-19)。
    final delete = find.widgetWithText(TextButton, l10n.photoDeleteButton);
    await tester.ensureVisible(delete);
    await tester.pump();
    await tester.tap(delete);
    for (var i = 0; i < 5; i++) {
      await tester.pump(const Duration(milliseconds: 10));
    }
    api.release('POST', '/api/purchase-suggestions');
    await tester.pumpAndSettle();

    // 削除した写真の推測を反映しない (FR-19)。
    expect(find.widgetWithText(TextField, 'Medium'), findsNothing);
    expect(find.byKey(const Key('purchase-photo')), findsNothing);
  });

  testWidgets('5 MB を超える写真を選び直すと、商品の登録の導線を消す', (tester) async {
    final l10n = await loadL10n();
    final picker = FakePhotoPicker(
      photo: PickedPhoto(name: 'package.png', bytes: Uint8List.fromList(<int>[1, 2, 3])),
    );
    final converter = FakeImageConverter(bytes: Uint8List.fromList(<int>[0xFF, 0xD8, 0xFF]));
    final api = await openPurchaseForm(tester, picker: picker, converter: converter);
    // 一致が無い推測を返し、商品の登録の導線を出す (FR-19)。
    onProductNameLookup(api, <Map<String, Object?>>[]);
    api.on(
      'POST',
      '/api/purchase-suggestions',
      status: 200,
      body: suggestionJson(product: suggestedProductJson(name: 'New Bean')),
    );

    await scrollAndTap(tester, find.widgetWithText(OutlinedButton, l10n.photoSelectButton));
    expect(
      find.widgetWithText(OutlinedButton, l10n.suggestionRegisterProductButton),
      findsOneWidget,
    );

    // 5 MB を超える写真を選び直すと、商品の登録の導線を消す (FR-19)。
    converter.bytes = Uint8List(maxPhotoBytes + 1);
    await scrollAndTap(tester, find.widgetWithText(OutlinedButton, l10n.photoReplaceButton));
    expect(
      find.widgetWithText(OutlinedButton, l10n.suggestionRegisterProductButton),
      findsNothing,
    );
  });

  testWidgets('変換の結果が 5 MB を超えるときは推測を呼ばない', (tester) async {
    final l10n = await loadL10n();
    final picker = FakePhotoPicker(
      photo: PickedPhoto(name: 'big.jpg', bytes: Uint8List.fromList(<int>[1])),
    );
    final api = await openPurchaseForm(
      tester,
      picker: picker,
      converter: FakeImageConverter(bytes: Uint8List(maxPhotoBytes + 1)),
    );

    await scrollAndTap(tester, find.widgetWithText(OutlinedButton, l10n.photoSelectButton));

    // 5 MB を超える写真では推測を呼ばない (FR-19)。
    expect(api.calls, isNot(contains('POST /api/purchase-suggestions')));
    expect(find.byKey(const Key('purchase-photo')), findsOneWidget);
  });

  testWidgets('2 段組で商品の登録から戻っても、写真と推測の反映と入力値が残る', (tester) async {
    // 幅 840 px 以上の 2 段組で立ち上げる (docs/design/components/WideLayout)。
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final l10n = await loadL10n();
    final picker = FakePhotoPicker(
      photo: PickedPhoto(name: 'package.png', bytes: Uint8List.fromList(<int>[1, 2, 3])),
    );
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
        body: pageJson(key: 'purchases', items: <Map<String, Object?>>[]),
      );
    onProductNameLookup(api, <Map<String, Object?>>[]);
    api
      ..on(
        'POST',
        '/api/purchase-suggestions',
        status: 200,
        body: suggestionJson(
          product: suggestedProductJson(name: 'New Bean', origin: 'Ethiopia'),
          roast: 'Medium',
          priceAmount: 1200,
        ),
      )
      ..on(
        'POST',
        '/api/products',
        status: 200,
        body: productJson(id: 'product-9', name: 'New Bean'),
      );
    await pumpApp(
      tester,
      apiClient: api.client(),
      passkeyClient: FakePasskeyClient(),
      photoPicker: picker,
      imageConverter: FakeImageConverter(bytes: Uint8List.fromList(<int>[0xFF, 0xD8, 0xFF])),
    );
    await openLocation(tester, AppRoutes.purchases);
    expect(find.byType(PurchaseListScreen), findsOneWidget);

    // 2 段組の右の面で購入を登録する。
    await tester.tap(find.widgetWithText(FloatingActionButton, l10n.newPurchaseButton));
    await tester.pumpAndSettle();
    expect(find.byType(PurchaseFormScreen), findsOneWidget);

    // 入力値と写真を用意し、推測を反映する。
    await scrollAndEnterText(tester, find.byType(TextField).at(1), 'Custom Roast');
    await scrollAndTap(tester, find.widgetWithText(OutlinedButton, l10n.photoSelectButton));

    // 押し出しの画面で商品を登録する (右の面は入れ替えない。FR-19)。
    await scrollAndTap(
      tester,
      find.widgetWithText(OutlinedButton, l10n.suggestionRegisterProductButton),
    );
    expect(find.byType(ProductFormScreen), findsOneWidget);
    await scrollAndTap(tester, find.widgetWithText(TextButton, l10n.saveButton));

    // 購入のフォームに戻り、選択した写真、推測の反映、入力値が残る (FR-19)。
    expect(find.byType(PurchaseFormScreen), findsOneWidget);
    expect(find.byKey(const Key('purchase-photo')), findsOneWidget);
    expect(find.widgetWithText(TextField, 'Custom Roast'), findsOneWidget);
    expect(find.widgetWithText(TextField, '1200'), findsOneWidget);
    expect(find.text('New Bean'), findsOneWidget);
  });
}
