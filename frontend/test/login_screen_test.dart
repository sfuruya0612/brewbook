import 'package:brew_book/auth/passkey_error.dart';
import 'package:brew_book/screens/home_screen.dart';
import 'package:brew_book/screens/login_screen.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/fake_api.dart';
import 'support/fake_records.dart';
import 'support/fake_passkey_client.dart';
import 'support/pump_app.dart';

void main() {
  /// ログイン画面を表示した状態にする (起動時の確認は 401 にする)。
  Future<FakeApi> showLogin(WidgetTester tester, {FakePasskeyClient? passkeys}) async {
    final api = FakeApi()..on('GET', '/api/passkeys', status: 401, body: unauthorizedBody);
    await pumpApp(tester, apiClient: api.client(), passkeyClient: passkeys ?? FakePasskeyClient());
    return api;
  }

  testWidgets('起動時の確認が 401 ならログイン画面を表示する', (tester) async {
    final l10n = await loadL10n();
    await showLogin(tester);

    expect(find.byType(LoginScreen), findsOneWidget);
    expect(find.text(l10n.loginTitle), findsOneWidget);
    expect(find.text(l10n.loginDescription), findsOneWidget);
  });

  testWidgets('利用者名とパスワードの入力は置かない', (tester) async {
    final l10n = await loadL10n();
    await showLogin(tester);

    expect(find.byType(TextField), findsNothing);
    expect(find.widgetWithText(FilledButton, l10n.loginButton), findsOneWidget);
  });

  testWidgets('パスキーでログインするとホームへ遷移する', (tester) async {
    final l10n = await loadL10n();
    final passkeys = FakePasskeyClient();
    final api = await showLogin(tester, passkeys: passkeys);
    api
      ..on(
        'POST',
        '/api/auth/login/begin',
        status: 200,
        body: <String, Object?>{'challenge': 'test-challenge', 'rpId': 'localhost'},
      )
      ..on(
        'POST',
        '/api/auth/login/complete',
        status: 200,
        body: <String, Object?>{'user_id': 'test-user'},
      )
      // ホームは抽出の一覧を読む (FR-11)。
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      );

    await tester.tap(find.widgetWithText(FilledButton, l10n.loginButton));
    await tester.pumpAndSettle();

    expect(find.byType(HomeScreen), findsOneWidget);
    expect(find.text(l10n.noRecords), findsOneWidget);
    expect(api.calls, <String>[
      'GET /api/passkeys',
      'POST /api/auth/login/begin',
      'POST /api/auth/login/complete',
      'GET /api/brews',
    ]);
    // サーバーのオプションはそのままパスキーのクライアントへ渡す。
    expect(passkeys.requestedOptions?['challenge'], 'test-challenge');
  });

  testWidgets('400 のときはログインの失敗を表示する', (tester) async {
    final l10n = await loadL10n();
    final api = await showLogin(tester);
    api
      ..on('POST', '/api/auth/login/begin', status: 200, body: <String, Object?>{})
      ..on('POST', '/api/auth/login/complete', status: 400, body: badRequestBody);

    await tester.tap(find.widgetWithText(FilledButton, l10n.loginButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.loginFailed), findsOneWidget);
    expect(find.byType(HomeScreen), findsNothing);
  });

  testWidgets('409 (チャレンジの再利用) のときはログインの失敗を表示する', (tester) async {
    final l10n = await loadL10n();
    final api = await showLogin(tester);
    api.on('POST', '/api/auth/login/begin', status: 409, body: conflictBody);

    await tester.tap(find.widgetWithText(FilledButton, l10n.loginButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.loginFailed), findsOneWidget);
  });

  testWidgets('401 のときはセッションの失効を表示する', (tester) async {
    final l10n = await loadL10n();
    final api = await showLogin(tester);
    api
      ..on('POST', '/api/auth/login/begin', status: 200, body: <String, Object?>{})
      ..on('POST', '/api/auth/login/complete', status: 401, body: unauthorizedBody);

    await tester.tap(find.widgetWithText(FilledButton, l10n.loginButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.errorUnauthorized), findsOneWidget);
    expect(find.byType(LoginScreen), findsOneWidget);
  });

  testWidgets('ネットワークエラーのときは再試行を促す', (tester) async {
    final l10n = await loadL10n();
    final api = await showLogin(tester);
    api.onNetworkError('POST', '/api/auth/login/begin');

    await tester.tap(find.widgetWithText(FilledButton, l10n.loginButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.errorNetwork), findsOneWidget);
  });

  testWidgets('パスキーの取り消しを表示する', (tester) async {
    final l10n = await loadL10n();
    final passkeys = FakePasskeyClient()..failure = PasskeyErrorKind.cancelled;
    final api = await showLogin(tester, passkeys: passkeys);
    api.on('POST', '/api/auth/login/begin', status: 200, body: <String, Object?>{});

    await tester.tap(find.widgetWithText(FilledButton, l10n.loginButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.passkeyCancelled), findsOneWidget);
  });

  testWidgets('パスキーを使えない環境ではその旨を表示する', (tester) async {
    final l10n = await loadL10n();
    final passkeys = FakePasskeyClient()..failure = PasskeyErrorKind.unsupported;
    final api = await showLogin(tester, passkeys: passkeys);
    api.on('POST', '/api/auth/login/begin', status: 200, body: <String, Object?>{});

    await tester.tap(find.widgetWithText(FilledButton, l10n.loginButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.passkeyUnsupported), findsOneWidget);
  });
}
