import 'passkey_client.dart';
import 'passkey_error.dart';

/// Web 以外 (iOS など) のパスキーの実装。
///
/// iOS の実装は配布形態が決まってから足す (ADR-0007)。それまでは、パスキーを使えないことを
/// [PasskeyError] の `unsupported` で伝える。
class UnsupportedPasskeyClient implements PasskeyClient {
  /// 実行環境でパスキーを使えないときのメッセージ (英語)。
  static const String message = 'the passkey client is not implemented on this platform';

  @override
  Future<Map<String, Object?>> createCredential(Map<String, Object?> options) {
    return Future<Map<String, Object?>>.error(
      const PasskeyError(PasskeyErrorKind.unsupported, message),
    );
  }

  @override
  Future<Map<String, Object?>> getCredential(Map<String, Object?> options) {
    return Future<Map<String, Object?>>.error(
      const PasskeyError(PasskeyErrorKind.unsupported, message),
    );
  }
}

/// Web 以外のプラットフォームの [PasskeyClient] を返す。
PasskeyClient createPasskeyClient() => UnsupportedPasskeyClient();
