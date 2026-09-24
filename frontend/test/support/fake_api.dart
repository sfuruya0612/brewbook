import 'dart:convert';

import 'package:coffee_log/api/api_client.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

/// 401 のエラー応答 (PRD の「エラー応答の規約」)。
const Map<String, Object?> unauthorizedBody = <String, Object?>{
  'error': <String, Object?>{'code': 'unauthorized', 'message': 'the session is not valid'},
};

/// 400 のエラー応答。
const Map<String, Object?> badRequestBody = <String, Object?>{
  'error': <String, Object?>{'code': 'bad_request', 'message': 'the input is not valid'},
};

/// 409 のエラー応答。
const Map<String, Object?> conflictBody = <String, Object?>{
  'error': <String, Object?>{'code': 'conflict', 'message': 'the state is in conflict'},
};

/// 410 のエラー応答。
const Map<String, Object?> goneBody = <String, Object?>{
  'error': <String, Object?>{'code': 'gone', 'message': 'the value has expired'},
};

/// テスト用の API。経路ごとの応答を登録し、呼び出しを記録する。
///
/// `ApiClient` は `package:http` のクライアントを差し替えられるため、本物の変換
/// (エラーの共通の型への変換、401 の通知) を通したまま応答を決められる。
class FakeApi {
  final Map<String, _FakeResponse> _responses = <String, _FakeResponse>{};

  /// 呼び出された経路 (例: `GET /api/passkeys`) の記録。
  final List<String> calls = <String>[];

  /// `method` と `path` (例: `POST`、`/api/auth/login/begin`) の応答を登録する。
  void on(String method, String path, {required int status, Object? body}) {
    _responses['$method $path'] = _FakeResponse(status: status, body: body);
  }

  /// `method` と `path` の呼び出しを接続の失敗にする。
  void onNetworkError(String method, String path) {
    _responses['$method $path'] = const _FakeResponse.network();
  }

  /// 登録した応答を返す [ApiClient] を組み立てる。
  ApiClient client() => ApiClient(httpClient: MockClient(_handle));

  Future<http.Response> _handle(http.Request request) async {
    final key = '${request.method} ${request.url.path}';
    calls.add(key);
    final response = _responses[key];
    if (response == null) {
      // 登録していない経路はテストの前提の誤りなので、原因が分かる応答にする。
      return _json(500, <String, Object?>{
        'error': <String, Object?>{'code': 'unexpected_call', 'message': key},
      });
    }
    if (response.network) {
      throw http.ClientException('the connection failed');
    }
    return _json(response.status, response.body);
  }

  static http.Response _json(int status, Object? body) {
    return http.Response(
      jsonEncode(body ?? const <String, Object?>{}),
      status,
      headers: const <String, String>{'Content-Type': 'application/json'},
    );
  }
}

/// テスト用の応答。
class _FakeResponse {
  const _FakeResponse({required this.status, this.body}) : network = false;

  const _FakeResponse.network() : status = 0, body = null, network = true;

  final int status;
  final Object? body;
  final bool network;
}
