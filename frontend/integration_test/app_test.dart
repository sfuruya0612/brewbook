import 'dart:convert';

import 'package:coffee_log/api/api_client.dart';
import 'package:coffee_log/app.dart';
import 'package:coffee_log/auth/passkey_client.dart';
import 'package:coffee_log/screens/login_screen.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:integration_test/integration_test.dart';

/// 統合テストのハーネスが動くことを確認するスモークテスト (frontend:test-integration)。
///
/// 実バックエンドと仮想認証器を使う検証は 0014 と 0017 が追加する。`flutter drive` の Web の
/// ビルドは `integration_test/` の外のテスト補助を読めないため、テスト用のクライアントは
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
}

/// スモークテストはパスキーを使わないため、呼ばれたら失敗させる。
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
