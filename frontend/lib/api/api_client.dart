import 'dart:convert';

import 'package:http/http.dart' as http;

import 'api_error.dart';

/// `/api` の下を呼ぶ API クライアント (ADR-0007)。
///
/// セッションはブラウザの Cookie が管理するため (ADR-0005)、このクライアントは Cookie を扱わない。
/// エラーの応答は共通の型 ([ApiError]) に、応答を取得できない失敗は [NetworkError] に変換する。
class ApiClient {
  ApiClient({http.Client? httpClient, this.basePath = defaultBasePath})
    : _http = httpClient ?? http.Client();

  /// 同一オリジンの `/api` を呼ぶ (ADR-0005)。
  static const String defaultBasePath = '/api';

  final http.Client _http;

  /// `/api` の手前までの経路。
  final String basePath;

  /// 401 の応答を受け取ったときに呼ぶ。ログイン画面へ遷移させるために使う (ADR-0007)。
  ///
  /// アプリの起動時に、セッションの監視 (AuthController の markSignedOut) を設定する。
  void Function()? onUnauthorized;

  /// `GET` を呼び、JSON のオブジェクトを返す。
  Future<Map<String, Object?>> getJson(String path) async {
    return _decodeJson(await _send(() => _http.get(_uri(path))));
  }

  /// `GET` を呼び、応答の本文をバイト列として返す (FR-14 のダウンロード)。
  ///
  /// 認証は同じオリジンの Cookie で行う (ADR-0005)。
  Future<List<int>> getBytes(String path) async {
    final response = await _send(() => _http.get(_uri(path)));
    return response.bodyBytes;
  }

  /// `POST` を呼び、JSON のオブジェクトを返す。本文が無いときは空のオブジェクトを送る。
  Future<Map<String, Object?>> postJson(String path, [Map<String, Object?>? body]) async {
    return _decodeJson(
      await _send(
        () => _http.post(
          _uri(path),
          headers: _jsonHeaders,
          body: jsonEncode(body ?? const <String, Object?>{}),
        ),
      ),
    );
  }

  /// `PATCH` を呼び、JSON のオブジェクトを返す。
  Future<Map<String, Object?>> patchJson(String path, Map<String, Object?> body) async {
    return _decodeJson(
      await _send(
        () => _http.patch(_uri(path), headers: _jsonHeaders, body: jsonEncode(body)),
      ),
    );
  }

  /// `DELETE` を呼び、JSON のオブジェクトを返す。
  Future<Map<String, Object?>> deleteJson(String path) async {
    return _decodeJson(await _send(() => _http.delete(_uri(path))));
  }

  /// ページのオリジンを基準にした絶対 URL にする。Web では同一オリジン、それ以外では
  /// `Uri.base` を基準にした URL になる。
  Uri _uri(String path) => Uri.base.resolve('$basePath$path');

  /// 要求を送り、エラーの応答を [ApiError] に、応答を取得できない失敗を [NetworkError] にする。
  Future<http.Response> _send(Future<http.Response> Function() request) async {
    final http.Response response;
    try {
      response = await request();
    } on Exception catch (error) {
      // 接続の失敗とタイムアウトは、再試行を促す表示にする (ADR-0007)。
      throw NetworkError(error);
    }
    if (response.statusCode >= 400) {
      final error = _errorFrom(response);
      if (error.isUnauthorized) {
        onUnauthorized?.call();
      }
      throw error;
    }
    return response;
  }

  /// エラーの応答を [ApiError] にする。規約の形でない応答はステータスコードだけを運ぶ。
  static ApiError _errorFrom(http.Response response) {
    final body = _tryDecode(response.body);
    final error = body?['error'];
    if (error is Map<String, Object?>) {
      final code = error['code'];
      final message = error['message'];
      if (code is String && message is String) {
        return ApiError(status: response.statusCode, code: code, message: message);
      }
    }
    return ApiError(
      status: response.statusCode,
      code: 'unknown',
      message: response.body,
    );
  }

  /// 成功の応答の本文を JSON のオブジェクトにする。空の本文は空のオブジェクトとする。
  static Map<String, Object?> _decodeJson(http.Response response) {
    if (response.body.isEmpty) {
      return const <String, Object?>{};
    }
    final body = _tryDecode(response.body);
    if (body == null) {
      throw NetworkError('the response body is not a JSON object: ${response.body}');
    }
    return body;
  }

  /// 本文を JSON のオブジェクトにする。JSON でないか、オブジェクトでなければ null を返す。
  static Map<String, Object?>? _tryDecode(String body) {
    if (body.isEmpty) {
      return null;
    }
    final Object? decoded;
    try {
      decoded = jsonDecode(body);
    } on FormatException {
      return null;
    }
    if (decoded is Map<String, Object?>) {
      return decoded;
    }
    return null;
  }

  static const Map<String, String> _jsonHeaders = <String, String>{
    'Content-Type': 'application/json',
  };
}
