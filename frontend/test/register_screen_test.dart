import 'package:brew_book/router/app_router.dart';
import 'package:brew_book/screens/home_screen.dart';
import 'package:brew_book/screens/login_screen.dart';
import 'package:brew_book/screens/register_screen.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';

import 'support/fake_api.dart';
import 'support/fake_records.dart';
import 'support/fake_passkey_client.dart';
import 'support/pump_app.dart';

void main() {
  /// 登録の画面を開く。アプリは起動時の確認が 401 の状態 (ログイン画面) にする。
  Future<FakeApi> openRegister(
    WidgetTester tester,
    String location, {
    FakePasskeyClient? passkeys,
  }) async {
    final api = FakeApi()..on('GET', '/api/passkeys', status: 401, body: unauthorizedBody);
    await pumpApp(
      tester,
      apiClient: api.client(),
      passkeyClient: passkeys ?? FakePasskeyClient(),
    );
    expect(find.byType(LoginScreen), findsOneWidget);
    GoRouter.of(tester.element(find.byType(LoginScreen))).go(location);
    await tester.pumpAndSettle();
    return api;
  }

  testWidgets('/register?token= を開くと登録の画面を表示する', (tester) async {
    final l10n = await loadL10n();
    await openRegister(tester, AppRoutes.registerWithToken('test-token'));

    expect(find.byType(RegisterScreen), findsOneWidget);
    expect(find.text(l10n.registerTitle), findsOneWidget);
    expect(find.text(l10n.registerDescription), findsOneWidget);
    // パスキーの名前だけを入力する。
    expect(find.byType(TextField), findsOneWidget);
    expect(find.widgetWithText(FilledButton, l10n.registerButton), findsOneWidget);
  });

  testWidgets('トークンが無いときはその旨を表示する', (tester) async {
    final l10n = await loadL10n();
    await openRegister(tester, AppRoutes.register);

    expect(find.byType(RegisterScreen), findsOneWidget);
    expect(find.text(l10n.registerTokenMissing), findsOneWidget);
    expect(find.byType(TextField), findsNothing);
  });

  testWidgets('名前が空のまま登録するとエラーを表示し、API を呼ばない', (tester) async {
    final l10n = await loadL10n();
    final api = await openRegister(tester, AppRoutes.registerWithToken('test-token'));

    await tester.tap(find.widgetWithText(FilledButton, l10n.registerButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.passkeyNameError), findsOneWidget);
    expect(api.calls, <String>['GET /api/passkeys']);
  });

  testWidgets('名前が 50 文字を超えるとエラーを表示する', (tester) async {
    final l10n = await loadL10n();
    final api = await openRegister(tester, AppRoutes.registerWithToken('test-token'));

    await tester.enterText(find.byType(TextField), 'a' * 51);
    await tester.tap(find.widgetWithText(FilledButton, l10n.registerButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.passkeyNameError), findsOneWidget);
    expect(api.calls, <String>['GET /api/passkeys']);
  });

  testWidgets('登録に成功するとホームへ遷移する', (tester) async {
    final l10n = await loadL10n();
    final passkeys = FakePasskeyClient();
    final api = await openRegister(
      tester,
      AppRoutes.registerWithToken('test-token'),
      passkeys: passkeys,
    );
    api
      ..on(
        'POST',
        '/api/auth/register/begin',
        status: 200,
        body: <String, Object?>{'challenge': 'test-challenge'},
      )
      ..on(
        'POST',
        '/api/auth/register/complete',
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

    await tester.enterText(find.byType(TextField), '自宅の PC');
    await tester.tap(find.widgetWithText(FilledButton, l10n.registerButton));
    await tester.pumpAndSettle();

    expect(find.byType(HomeScreen), findsOneWidget);
    expect(api.calls, <String>[
      'GET /api/passkeys',
      'POST /api/auth/register/begin',
      'POST /api/auth/register/complete',
      'GET /api/brews',
    ]);
    expect(passkeys.createdOptions?['challenge'], 'test-challenge');
  });

  testWidgets('使用済みのトークン (409) を表示する', (tester) async {
    final l10n = await loadL10n();
    final api = await openRegister(tester, AppRoutes.registerWithToken('test-token'));
    api.on('POST', '/api/auth/register/begin', status: 409, body: conflictBody);

    await tester.enterText(find.byType(TextField), '自宅の PC');
    await tester.tap(find.widgetWithText(FilledButton, l10n.registerButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.registerTokenUsed), findsOneWidget);
  });

  testWidgets('期限切れのトークン (410) を表示する', (tester) async {
    final l10n = await loadL10n();
    final api = await openRegister(tester, AppRoutes.registerWithToken('test-token'));
    api.on('POST', '/api/auth/register/begin', status: 410, body: goneBody);

    await tester.enterText(find.byType(TextField), '自宅の PC');
    await tester.tap(find.widgetWithText(FilledButton, l10n.registerButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.registerTokenExpired), findsOneWidget);
  });

  testWidgets('存在しないトークン (404) を表示する', (tester) async {
    final l10n = await loadL10n();
    final api = await openRegister(tester, AppRoutes.registerWithToken('test-token'));
    api.on(
      'POST',
      '/api/auth/register/begin',
      status: 404,
      body: <String, Object?>{
        'error': <String, Object?>{'code': 'not_found', 'message': 'the token does not exist'},
      },
    );

    await tester.enterText(find.byType(TextField), '自宅の PC');
    await tester.tap(find.widgetWithText(FilledButton, l10n.registerButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.registerTokenNotFound), findsOneWidget);
  });

  testWidgets('クレデンシャルの検証の失敗 (400) を表示する', (tester) async {
    final l10n = await loadL10n();
    final api = await openRegister(tester, AppRoutes.registerWithToken('test-token'));
    api
      ..on(
        'POST',
        '/api/auth/register/begin',
        status: 200,
        body: <String, Object?>{'challenge': 'test-challenge'},
      )
      // 400 は名前の不正と検証の失敗のどちらでも返るため、汎用の文言を表示する。
      ..on('POST', '/api/auth/register/complete', status: 400, body: badRequestBody);

    await tester.enterText(find.byType(TextField), '自宅の PC');
    await tester.tap(find.widgetWithText(FilledButton, l10n.registerButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.errorValidation), findsOneWidget);
    expect(api.calls, <String>[
      'GET /api/passkeys',
      'POST /api/auth/register/begin',
      'POST /api/auth/register/complete',
    ]);
  });

  testWidgets('名前が 50 文字ちょうどなら登録できる', (tester) async {
    final l10n = await loadL10n();
    final api = await openRegister(tester, AppRoutes.registerWithToken('test-token'));
    api
      ..on(
        'POST',
        '/api/auth/register/begin',
        status: 200,
        body: <String, Object?>{'challenge': 'test-challenge'},
      )
      ..on(
        'POST',
        '/api/auth/register/complete',
        status: 200,
        body: <String, Object?>{'user_id': 'test-user'},
      );

    await tester.enterText(find.byType(TextField), 'a' * 50);
    await tester.tap(find.widgetWithText(FilledButton, l10n.registerButton));
    await tester.pumpAndSettle();

    expect(find.byType(HomeScreen), findsOneWidget);
  });

  testWidgets('名前が空白だけならエラーを表示し、API を呼ばない', (tester) async {
    final l10n = await loadL10n();
    final api = await openRegister(tester, AppRoutes.registerWithToken('test-token'));

    await tester.enterText(find.byType(TextField), '   ');
    await tester.tap(find.widgetWithText(FilledButton, l10n.registerButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.passkeyNameError), findsOneWidget);
    // 起動時の確認だけが呼ばれる。
    expect(api.calls, <String>['GET /api/passkeys']);
  });

  testWidgets('ネットワークエラーのときは再試行を促す', (tester) async {
    final l10n = await loadL10n();
    final api = await openRegister(tester, AppRoutes.registerWithToken('test-token'));
    api.onNetworkError('POST', '/api/auth/register/begin');

    await tester.enterText(find.byType(TextField), '自宅の PC');
    await tester.tap(find.widgetWithText(FilledButton, l10n.registerButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.errorNetwork), findsOneWidget);
  });
}
