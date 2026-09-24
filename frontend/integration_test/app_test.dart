import 'dart:convert';

import 'package:coffee_log/api/api_client.dart';
import 'package:coffee_log/app.dart';
import 'package:coffee_log/auth/passkey_client.dart';
import 'package:coffee_log/l10n/app_localizations.dart';
import 'package:coffee_log/screens/login_screen.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:integration_test/integration_test.dart';

import 'fake_records.dart';

/// 統合テスト (frontend:test-integration)。
///
/// 実バックエンドと仮想認証器を使う検証は 0017 が追加する。ここではテスト用の API クライアントを
/// 差し込んで、画面の経路と画面数の成功指標 (PRD の成功指標) を検証する。`flutter drive` の
/// Web のビルドは `integration_test/` の外のテスト補助を読めないため、テスト用のクライアントは
/// ここで組み立てる。
void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('起動するとログイン画面を表示する', (tester) async {
    final api = ApiClient(
      httpClient: MockClient((request) async {
        // セッションが無い応答にして、ログイン画面へ遷移させる (FR-2)。
        return http.Response(
          jsonEncode(<String, Object?>{
            'error': <String, Object?>{
              'code': 'unauthorized',
              'message': 'the session is not valid',
            },
          }),
          401,
          headers: const <String, String>{'Content-Type': 'application/json'},
        );
      }),
    );
    await tester.pumpWidget(
      CoffeeLogApp(apiClient: api, passkeyClient: _UnusedPasskeyClient()),
    );
    await tester.pumpAndSettle();

    expect(find.byType(LoginScreen), findsOneWidget);
  });

  testWidgets('ホームから抽出の保存完了までの最短経路の画面は 3 つ以内である', (tester) async {
    // 表示の言語を固定して、文言で操作できるようにする (FR-16)。
    tester.platformDispatcher.localesTestValue = const <Locale>[Locale('en')];
    addTearDown(tester.platformDispatcher.clearLocalesTestValue);
    final l10n = await AppLocalizations.delegate.load(const Locale('en'));

    final observer = _ScreenCountObserver();
    final created = brewJson(
      id: 'brew-1',
      purchase: purchaseJson(product: productJson(name: 'Test Product')),
    );
    var saved = false;
    final api = ApiClient(
      httpClient: MockClient((request) async {
        final key = '${request.method} ${request.url.path}';
        if (key == 'POST /api/brews') {
          saved = true;
          return _json(200, created);
        }
        final body = switch (key) {
          'GET /api/passkeys' => <String, Object?>{'passkeys': <Object?>[]},
          // ホームの抽出一覧 (FR-11)。
          'GET /api/brews' => pageJson(
            key: 'brews',
            items: <Map<String, Object?>>[if (saved) created],
          ),
          // 抽出の登録の画面が開く購入の一覧 (FR-11)。
          'GET /api/purchases' => pageJson(
            key: 'purchases',
            items: <Map<String, Object?>>[purchaseJson()],
          ),
          _ => null,
        };
        if (body == null) {
          return _json(500, <String, Object?>{
            'error': <String, Object?>{'code': 'unexpected_call', 'message': key},
          });
        }
        return _json(200, body);
      }),
    );
    await tester.pumpWidget(
      CoffeeLogApp(
        apiClient: api,
        passkeyClient: _UnusedPasskeyClient(),
        navigatorObservers: <NavigatorObserver>[observer],
      ),
    );
    await tester.pumpAndSettle();

    // 最短経路: ホームで「抽出を記録」を押し、購入を選び、保存する。
    expect(find.text(l10n.newBrewButton), findsOneWidget);
    await tester.tap(find.text(l10n.newBrewButton));
    await tester.pumpAndSettle();

    await tester.tap(find.text(l10n.purchaseLabel));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Test Product').last);
    await tester.pumpAndSettle();

    await tester.tap(find.text(l10n.saveButton));
    await tester.pumpAndSettle();

    // 保存の完了は通知で示す (画面には数えない)。
    expect(find.text(l10n.savedMessage), findsOneWidget);
    // 表示した画面の種類は、ホームを含めて 3 つ以内である (PRD の成功指標)。
    expect(
      observer.screens,
      hasLength(lessThanOrEqualTo(3)),
      reason: 'visited=${observer.screens}',
    );
    // ホームと抽出の登録の画面の 2 つを通る。
    expect(observer.screens, containsAll(<String>['/', '/brews/new']));
  });
}

/// 状態コードと本文の応答を組み立てる。
http.Response _json(int status, Map<String, Object?> body) {
  return http.Response(
    jsonEncode(body),
    status,
    headers: const <String, String>{'Content-Type': 'application/json'},
  );
}

/// ルーターに登録した画面 (経路の名前) の表示を数える (PRD の成功指標)。
///
/// ダイアログ、ボトムシート、保存完了の通知はルーターの画面ではないため、数えない。
class _ScreenCountObserver extends NavigatorObserver {
  /// 表示した画面の名前 (経路のパターン)。重複は数えない。
  final Set<String> screens = <String>{};

  @override
  void didPush(Route<dynamic> route, Route<dynamic>? previousRoute) {
    final name = route.settings.name;
    if (name != null) {
      screens.add(name);
    }
  }
}

/// 統合テストはパスキーを使わないため、呼ばれたら失敗させる。
class _UnusedPasskeyClient implements PasskeyClient {
  @override
  Future<Map<String, Object?>> createCredential(Map<String, Object?> options) {
    throw UnsupportedError('the passkey client is not used in the smoke test');
  }

  @override
  Future<Map<String, Object?>> getCredential(Map<String, Object?> options) {
    throw UnsupportedError('the passkey client is not used in the smoke test');
  }
}
