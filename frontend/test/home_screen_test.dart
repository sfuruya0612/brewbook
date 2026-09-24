import 'package:coffee_log/screens/home_screen.dart';
import 'package:coffee_log/screens/login_screen.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/fake_api.dart';
import 'support/fake_passkey_client.dart';
import 'support/pump_app.dart';

void main() {
  testWidgets('起動時の確認が成功するとホームを表示する', (tester) async {
    final l10n = await loadL10n();
    final api = FakeApi()
      ..on('GET', '/api/passkeys', status: 200, body: <String, Object?>{'passkeys': <Object?>[]});
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());

    expect(find.byType(HomeScreen), findsOneWidget);
    expect(find.text(l10n.homeDescription), findsOneWidget);
    expect(find.widgetWithText(TextButton, l10n.logoutButton), findsOneWidget);
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

    api.on('GET', '/api/passkeys', status: 200, body: <String, Object?>{'passkeys': <Object?>[]});
    await tester.tap(find.widgetWithText(TextButton, l10n.retryButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.homeDescription), findsOneWidget);
    expect(api.calls, <String>['GET /api/passkeys', 'GET /api/passkeys']);
  });

  testWidgets('ログアウトするとログイン画面へ遷移する', (tester) async {
    final l10n = await loadL10n();
    final api = FakeApi()
      ..on('GET', '/api/passkeys', status: 200, body: <String, Object?>{'passkeys': <Object?>[]})
      ..on('POST', '/api/auth/logout', status: 200, body: <String, Object?>{});
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());

    await tester.tap(find.widgetWithText(TextButton, l10n.logoutButton));
    await tester.pumpAndSettle();

    expect(find.byType(LoginScreen), findsOneWidget);
    expect(api.calls, <String>['GET /api/passkeys', 'POST /api/auth/logout']);
  });

  testWidgets('ログアウトがネットワークエラーなら再試行を促す', (tester) async {
    final l10n = await loadL10n();
    final api = FakeApi()
      ..on('GET', '/api/passkeys', status: 200, body: <String, Object?>{'passkeys': <Object?>[]});
    api.onNetworkError('POST', '/api/auth/logout');
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());

    await tester.tap(find.widgetWithText(TextButton, l10n.logoutButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.errorNetwork), findsOneWidget);
    expect(find.byType(HomeScreen), findsOneWidget);
  });
}
