//! 実バックエンドと仮想認証器を使う統合テスト (0017)。
//!
//! 画面は Worker の Static Assets から配信され、API は同じオリジンの `/api` を呼ぶ (ADR-0005)。
//! パスキーは chromedriver が起動した Chrome の仮想認証器で作り、使う (PRD の制約:
//! パスキーを伴う統合テストは Chrome DevTools Protocol の仮想認証器で行う)。
//! 利用者と登録用トークンと、抽出の保存に使う店と商品と購入は、テストの準備として
//! ローカルの D1 に直接投入する (Rust のハーネス)。
//!
//! 起動の URL は `/register?token=<トークン>` とする。Flutter のルーティングのパスを直接開くと、
//! Worker の Static Assets が `index.html` を 200 で返し、登録の画面が表示される (完了条件)。
//! Rust のハーネス (`tests/wrangler_same_origin_e2e.rs`) が `wrangler dev` と chromedriver を起動し、
//! `flutter drive` でこのテストを実行する。

import 'package:brew_book/l10n/app_localizations.dart';
import 'package:brew_book/main.dart' as app;
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';

/// 下ごしらえした商品の名前 (Rust のハーネスと同じ値)。
const String productName = 'E2E Product';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('ルーティングのパスを直接開き、登録とログインから抽出の保存までを実行する', (tester) async {
    // 文言で操作できるように、表示の言語を固定する (FR-16)。
    tester.platformDispatcher.localesTestValue = const <Locale>[Locale('en')];
    addTearDown(tester.platformDispatcher.clearLocalesTestValue);
    final l10n = await AppLocalizations.delegate.load(const Locale('en'));

    // ウィジェットテストの binding は `defaultRouteName` を '/' に固定する (`TestWidgetsFlutterBinding`)。
    // 本番はブラウザが開いた URL のパスから初期の経路を決めるため、固定を外して実際の URL を使わせる
    // (パス形式の URL にするのは `lib/main.dart` の `usePathUrlStrategy`)。
    tester.platformDispatcher.clearDefaultRouteNameTestValue();

    // ブラウザは Worker の Flutter のルーティングのパスを直接開いている (完了条件)。
    // Worker の Static Assets が `index.html` を返し、アプリはそのパスで起動する。
    expect(Uri.base.path, '/register', reason: 'the browser must open the SPA path');
    expect(Uri.base.queryParameters['token'], isNotEmpty, reason: 'the URL must carry the token');

    app.main();

    // 起動の URL のパスは `/register` である。ルーティングのパスを直接開いても
    // 登録の画面が表示される (完了条件)。
    await waitFor(tester, find.text(l10n.registerTitle));

    // 登録用トークンでパスキーを登録する (FR-1)。仮想認証器がクレデンシャルを作る。
    // `IntegrationTestWidgetsFlutterBinding` はテスト用の入力の仕組みを登録しないため
    // (registerTestTextInput が false)、`tester.enterText` が使う入力の経路を明示的に登録する。
    tester.testTextInput.register();
    addTearDown(tester.testTextInput.unregister);
    await tester.enterText(find.byType(TextField), 'e2e-passkey');
    await tester.pump();
    await tester.tap(find.text(l10n.registerButton));
    // 登録が成功するとセッションが発行され、ホームへ遷移する。
    await waitFor(tester, find.text(l10n.newBrewButton));

    // ログアウトしてから、同じパスキーでログインする (FR-2、FR-4)。
    // ログアウトはホームのメニューの区切りの下に置く (AppBar のガイドライン)。
    await tester.tap(find.byTooltip(l10n.menuTooltip));
    await waitFor(tester, find.text(l10n.logoutButton));
    // メニューの開くアニメーションが終わってから押す (途中では位置が定まらない)。
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.logoutButton).last);
    await waitFor(tester, find.text(l10n.loginButton));
    await tester.tap(find.text(l10n.loginButton));
    await waitFor(tester, find.text(l10n.newBrewButton));

    // 抽出を保存する (FR-11)。購入は下ごしらえしたものを選ぶ。
    await tester.tap(find.text(l10n.newBrewButton));
    await waitFor(tester, find.text(l10n.purchaseLabel));
    await tester.tap(find.text(l10n.purchaseLabel));
    await waitFor(tester, find.text(productName));
    await tester.tap(find.text(productName).last);
    await waitFor(tester, find.text(l10n.saveButton));
    // 保存のボタンは画面の下にあり、そのままでは見えないため、先に画面内へ入れる。
    await tester.ensureVisible(find.text(l10n.saveButton));
    await tester.pump();
    await tester.tap(find.text(l10n.saveButton));
    await waitFor(tester, find.text(l10n.savedMessage));
    // 保存した抽出がホームの一覧に出る (抽出の保存まで)。
    await waitFor(tester, find.text(productName));
  });
}

/// ウィジェットが現れるまで、フレームを進めながら待つ。
///
/// 実バックエンドの応答と WebAuthn の呼び出しは実時間がかかるため、`pumpAndSettle` では待てない
/// (画面が静止したまま応答を待つことがある)。
Future<void> waitFor(
  WidgetTester tester,
  Finder finder, {
  Duration timeout = const Duration(seconds: 60),
}) async {
  final deadline = DateTime.now().add(timeout);
  while (DateTime.now().isBefore(deadline)) {
    if (finder.evaluate().isNotEmpty) {
      return;
    }
    await tester.pump(const Duration(milliseconds: 100));
  }
  // 失敗の原因を切り分けられるように、そのときの URL と表示されている文言も報告する。
  final texts = tester
      .widgetList<Text>(find.byType(Text))
      .map((widget) => widget.data)
      .whereType<String>()
      .toList();
  IntegrationTestWidgetsFlutterBinding.instance.reportData = <String, Object?>{
    'url': Uri.base.toString(),
    'defaultRouteName': WidgetsBinding.instance.platformDispatcher.defaultRouteName,
    'finder': finder.toString(),
    'visibleTexts': texts,
  };
  fail('the widget did not appear within $timeout: $finder\nvisible texts: $texts');
}
