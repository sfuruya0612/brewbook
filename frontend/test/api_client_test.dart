import 'package:coffee_log/api/api_client.dart';
import 'package:coffee_log/api/api_error.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

void main() {
  test('成功の応答の JSON を返す', () async {
    final client = ApiClient(
      httpClient: MockClient((request) async {
        expect(request.url.path, '/api/passkeys');
        expect(request.method, 'GET');
        return http.Response('{"passkeys": []}', 200);
      }),
    );
    expect(await client.getJson('/passkeys'), <String, Object?>{'passkeys': <Object?>[]});
  });

  test('空の本文は空の JSON として扱う', () async {
    final client = ApiClient(
      httpClient: MockClient((request) async => http.Response('', 200)),
    );
    expect(await client.postJson('/auth/logout'), isEmpty);
  });

  test('エラーの応答を共通の型にする', () async {
    final client = ApiClient(
      httpClient: MockClient((request) async {
        return http.Response(
          '{"error": {"code": "bad_request", "message": "the input is not valid"}}',
          400,
        );
      }),
    );
    await expectLater(
      client.postJson('/auth/login/begin'),
      throwsA(
        isA<ApiError>()
            .having((error) => error.status, 'status', 400)
            .having((error) => error.code, 'code', 'bad_request')
            .having((error) => error.message, 'message', 'the input is not valid'),
      ),
    );
  });

  test('規約の形でないエラーの応答はステータスコードだけを運ぶ', () async {
    final client = ApiClient(
      httpClient: MockClient((request) async => http.Response('internal error', 500)),
    );
    await expectLater(
      client.getJson('/passkeys'),
      throwsA(
        isA<ApiError>()
            .having((error) => error.status, 'status', 500)
            .having((error) => error.code, 'code', 'unknown'),
      ),
    );
  });

  test('401 で onUnauthorized を呼ぶ', () async {
    var notified = 0;
    final client = ApiClient(
      httpClient: MockClient((request) async {
        return http.Response(
          '{"error": {"code": "unauthorized", "message": "the session is not valid"}}',
          401,
        );
      }),
    )..onUnauthorized = () => notified++;

    await expectLater(client.getJson('/passkeys'), throwsA(isA<ApiError>()));
    expect(notified, 1);
  });

  test('接続の失敗は NetworkError にする', () async {
    final client = ApiClient(
      httpClient: MockClient((request) async {
        throw http.ClientException('the connection failed');
      }),
    );
    await expectLater(client.getJson('/passkeys'), throwsA(isA<NetworkError>()));
  });

  test('成功の応答が JSON でなければ NetworkError にする', () async {
    final client = ApiClient(
      httpClient: MockClient((request) async => http.Response('<html></html>', 200)),
    );
    await expectLater(client.getJson('/passkeys'), throwsA(isA<NetworkError>()));
  });

  test('POST は JSON の本文を送る', () async {
    final client = ApiClient(
      httpClient: MockClient((request) async {
        expect(request.headers['Content-Type'], 'application/json');
        expect(request.body, '{"token":"test-token"}');
        return http.Response('{}', 200);
      }),
    );
    await client.postJson('/auth/register/begin', <String, Object?>{'token': 'test-token'});
  });
}
