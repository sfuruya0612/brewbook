import 'package:flutter/foundation.dart';

import '../api/api_client.dart';
import '../api/api_error.dart';
import 'passkey_client.dart';

/// ログインの状態 (FR-1、FR-2、FR-4)。
enum SessionStatus {
  /// 起動時の確認中。
  checking,

  /// 起動時の確認が失敗し (401 以外の API エラー、ネットワークエラー)、ログイン状態が分からない。
  unknown,

  /// セッションが無いか無効。
  signedOut,

  /// セッションが有効。
  signedIn,
}

/// セッションとパスキーの操作を持つ [ChangeNotifier] (ADR-0007)。
///
/// セッションの確認は専用の API を設けず、起動時に `GET /api/passkeys` を呼んで 401 かどうかで
/// 判定する。セッションはブラウザの Cookie が管理するため (ADR-0005)、このクラスは Cookie を扱わない。
class AuthController extends ChangeNotifier {
  AuthController({required ApiClient apiClient, required PasskeyClient passkeyClient})
    : _api = apiClient,
      _passkeys = passkeyClient;

  final ApiClient _api;
  final PasskeyClient _passkeys;

  SessionStatus _status = SessionStatus.checking;

  /// `unknown` になった原因の例外 (画面の文言に使う)。成功したら消す。
  Object? _unknownError;

  /// 現在のログインの状態。
  SessionStatus get status => _status;

  /// `unknown` になった原因の例外。無ければ null。
  Object? get unknownError => _unknownError;

  /// 起動時に `GET /api/passkeys` を呼び、ログイン状態を判定する。
  ///
  /// 401 はログイン画面へ遷移させる。ネットワークエラーはログイン状態が分からないため、
  /// 再試行を促す表示にする (ADR-0007)。
  Future<void> check() async {
    _unknownError = null;
    _set(SessionStatus.checking);
    try {
      await _api.getJson('/passkeys');
      _set(SessionStatus.signedIn);
    } on ApiError catch (error) {
      // 401 だけを「セッション無し」とする。500 などの一時的な失敗でログイン画面へ
      // 飛ばさず、状態が分からないものとして再試行を促す表示にする。
      if (error.isUnauthorized) {
        _set(SessionStatus.signedOut);
      } else {
        _unknownError = error;
        _set(SessionStatus.unknown);
      }
    } on NetworkError catch (error) {
      _unknownError = error;
      _set(SessionStatus.unknown);
    }
  }

  /// パスキーでログインする (FR-2)。利用者名とパスワードは受け取らない。
  Future<void> login() async {
    final options = await _api.postJson('/auth/login/begin');
    final credential = await _passkeys.getCredential(options);
    await _api.postJson('/auth/login/complete', <String, Object?>{
      'credential': credential,
    });
    _set(SessionStatus.signedIn);
  }

  /// 登録用トークンでパスキーを登録する (FR-1)。成功するとセッションが発行される。
  Future<void> register({required String token, required String name}) async {
    final options = await _api.postJson('/auth/register/begin', <String, Object?>{
      'token': token,
    });
    final credential = await _passkeys.createCredential(options);
    await _api.postJson('/auth/register/complete', <String, Object?>{
      'token': token,
      'name': name,
      'credential': credential,
    });
    _set(SessionStatus.signedIn);
  }

  /// ログアウトする (FR-4)。ログアウト後はログイン画面へ遷移させる。
  Future<void> logout() async {
    await _api.postJson('/auth/logout');
    _set(SessionStatus.signedOut);
  }

  /// 401 の応答でセッションが失われたことを記録する。ログイン画面へ遷移させる (ADR-0007)。
  void markSignedOut() => _set(SessionStatus.signedOut);

  void _set(SessionStatus status) {
    if (_status == status) {
      return;
    }
    _status = status;
    notifyListeners();
  }
}
