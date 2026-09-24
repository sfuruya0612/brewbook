import 'passkey_client_stub.dart'
    if (dart.library.js_interop) 'passkey_client_web.dart' as implementation;

/// パスキー (WebAuthn) のクライアント (ADR-0004、ADR-0007)。
///
/// Web の実装は `navigator.credentials` を JS 相互運用で呼び、サーバーのオプションの JSON を
/// `navigator.credentials` が受け取る形に変換し、サーバーが検証するクレデンシャルの JSON を返す。
/// iOS の実装は配布形態が決まってから足すため、このインターフェースの後ろに置く (ADR-0007)。
///
/// オプションとクレデンシャルは、`/api` の JSON と同じ形
/// (`Map<String, Object?>`) で受け渡しする。
abstract class PasskeyClient {
  /// 登録の作成のオプション (サーバーの `POST /api/auth/register/begin` の応答) でパスキーを作る。
  ///
  /// 返す値は `POST /api/auth/register/complete` の `credential` に渡す。
  Future<Map<String, Object?>> createCredential(Map<String, Object?> options);

  /// ログインの要求のオプション (サーバーの `POST /api/auth/login/begin` の応答) でパスキーを使う。
  ///
  /// 返す値は `POST /api/auth/login/complete` の `credential` に渡す。
  Future<Map<String, Object?>> getCredential(Map<String, Object?> options);
}

/// 実行環境で使える [PasskeyClient] を返す。
///
/// Web は `navigator.credentials` の実装、Web 以外は未実装の実装になる (ADR-0007)。
PasskeyClient createPasskeyClient() => implementation.createPasskeyClient();
