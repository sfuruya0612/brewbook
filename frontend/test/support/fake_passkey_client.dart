import 'package:brew_book/auth/passkey_client.dart';
import 'package:brew_book/auth/passkey_error.dart';

/// テスト用のパスキーのクライアント。実際の `navigator.credentials` は呼ばない。
class FakePasskeyClient implements PasskeyClient {
  /// 登録とログインで返すクレデンシャル (サーバーに渡す JSON の形)。
  Map<String, Object?> credential = const <String, Object?>{
    'id': 'test-credential',
    'type': 'public-key',
    'response': <String, Object?>{
      'clientDataJSON': 'test-client-data',
      'attestationObject': 'test-attestation',
      'authenticatorData': 'test-authenticator-data',
      'signature': 'test-signature',
    },
  };

  /// 失敗させるときの種類。null なら成功する。
  PasskeyErrorKind? failure;

  /// 受け取った登録のオプション。
  Map<String, Object?>? createdOptions;

  /// 受け取ったログインのオプション。
  Map<String, Object?>? requestedOptions;

  @override
  Future<Map<String, Object?>> createCredential(Map<String, Object?> options) async {
    createdOptions = options;
    if (failure != null) {
      throw PasskeyError(failure!, 'the test requested a failure');
    }
    return credential;
  }

  @override
  Future<Map<String, Object?>> getCredential(Map<String, Object?> options) async {
    requestedOptions = options;
    if (failure != null) {
      throw PasskeyError(failure!, 'the test requested a failure');
    }
    return credential;
  }
}
