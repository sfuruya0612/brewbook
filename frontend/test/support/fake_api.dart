import 'dart:async';
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

/// ログイン済みの状態で立ち上げるための API (起動時の `GET /api/passkeys` が 200)。
FakeApi signedInApi() {
  return FakeApi()
    ..on('GET', '/api/passkeys', status: 200, body: <String, Object?>{'passkeys': <Object?>[]});
}

/// テスト用の API。経路ごとの応答を登録し、呼び出しを記録する。
///
/// `ApiClient` は `package:http` のクライアントを差し替えられるため、本物の変換
/// (エラーの共通の型への変換、401 の通知) を通したまま応答を決められる。
class FakeApi {
  final Map<String, _FakeResponse> _responses = <String, _FakeResponse>{};
  final Map<String, ({int status, Object? body}) Function(Map<String, String> query)>
  _queryResponses =
      <String, ({int status, Object? body}) Function(Map<String, String> query)>{};

  /// 呼び出された経路 (例: `GET /api/passkeys`) の記録。クエリパラメータは含めない。
  final List<String> calls = <String>[];

  /// 受け取った要求 (本文の確認に使う)。
  final List<http.Request> requests = <http.Request>[];

  /// `method` と `path` (例: `POST`、`/api/auth/login/begin`) の応答を登録する。
  void on(String method, String path, {required int status, Object? body}) {
    _responses['$method $path'] = _FakeResponse(status: status, body: body);
  }

  /// クエリパラメータに応じて応答を変える登録 (一覧の追加読み込みなど)。
  ///
  /// `respond` はクエリパラメータを受け取り、状態コードと本文の組を返す。
  void onQuery(
    String method,
    String path,
    ({int status, Object? body}) Function(Map<String, String> query) respond,
  ) {
    _queryResponses['$method $path'] = respond;
  }

  /// 次に届く `method` と `path` の応答を保留する予約。キーは `method path`。
  final Map<String, Completer<void>> _holds = <String, Completer<void>>{};

  /// 実際に待っている応答。`release` が完了させる。
  final Map<String, Completer<void>> _held = <String, Completer<void>>{};

  /// 次に届く `method` と `path` の応答を 1 回だけ保留する。
  ///
  /// 期間の切り替えのように読み込みが重なる状況をテストで作るのに使う。
  /// 2 回目以降の要求は保留しない (テストが古い応答だけを解放できるようにするため)。
  void holdOnce(String method, String path) {
    _holds['$method $path'] = Completer<void>();
  }

  /// 保留した応答を解放する。
  void release(String method, String path) {
    final key = '$method $path';
    (_holds.remove(key) ?? _held.remove(key))?.complete();
  }

  /// `method` と `path` の呼び出しを接続の失敗にする。
  void onNetworkError(String method, String path) {
    _responses['$method $path'] = const _FakeResponse.network();
  }

  /// 登録した応答を返す [ApiClient] を組み立てる。
  ApiClient client() => ApiClient(httpClient: MockClient(_handle));

  /// `method` と `path` に一致する直近の要求の本文を JSON として返す。無ければ null。
  Map<String, Object?>? lastBody(String method, String path) {
    for (final request in requests.reversed) {
      if ('${request.method} ${request.url.path}' == '$method $path') {
        final decoded = jsonDecode(request.body);
        return decoded is Map<String, Object?> ? decoded : null;
      }
    }
    return null;
  }

  /// `method` と `path` に一致する直近の要求のクエリパラメータを返す。無ければ null。
  ///
  /// 統計の API に期間と粒度と UTC オフセットが渡ることを確認するのに使う (FR-18)。
  Map<String, String>? lastQuery(String method, String path) {
    for (final request in requests.reversed) {
      if ('${request.method} ${request.url.path}' == '$method $path') {
        return request.url.queryParameters;
      }
    }
    return null;
  }

  Future<http.Response> _handle(http.Request request) async {
    final key = '${request.method} ${request.url.path}';
    calls.add(key);
    requests.add(request);
    final hold = _holds.remove(key);
    if (hold != null) {
      // 待っている間も `release` が完了できるよう、別の置き場に移してから待つ。
      _held[key] = hold;
      await hold.future;
      _held.remove(key);
    }
    final respond = _queryResponses[key];
    if (respond != null) {
      final result = respond(request.url.queryParameters);
      return _json(result.status, result.body);
    }
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
