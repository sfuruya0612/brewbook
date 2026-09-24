import 'dart:convert';
import 'dart:js_interop';
import 'dart:js_interop_unsafe';
import 'dart:typed_data';

import 'package:flutter/foundation.dart' show visibleForTesting;
import 'package:web/web.dart' as web;

import 'passkey_client.dart';
import 'passkey_error.dart';

/// Web のパスキーの実装。`navigator.credentials` を `dart:js_interop` と `package:web` で呼ぶ (ADR-0007)。
///
/// サーバーのオプションの JSON を JS のオプションに変換して呼び出し、応答を base64url の JSON にする。
/// base64url はパディングを付けない (W3C WebAuthn Level 3 の 3)。
class WebPasskeyClient implements PasskeyClient {
  @override
  Future<Map<String, Object?>> createCredential(Map<String, Object?> options) async {
    final publicKey = web.PublicKeyCredentialCreationOptions(
      challenge: _bufferSource(options['challenge']),
      rp: _relyingParty(options['rp']),
      user: _userEntity(options['user']),
      pubKeyCredParams: _algorithms(options['pubKeyCredParams']),
      timeout: _timeout(options['timeout']),
      attestation: options['attestation'] as String? ?? 'none',
      authenticatorSelection: _authenticatorSelection(options['authenticatorSelection']),
    );
    final credential = await _call(
      () => web.window.navigator.credentials
          .create(web.CredentialCreationOptions(publicKey: publicKey))
          .toDart,
    );
    if (credential == null) {
      // 利用者が選択を取り消すと null が返る (W3C WebAuthn Level 3 の 5.1.4)。
      throw const PasskeyError(PasskeyErrorKind.cancelled, 'the credential is null');
    }
    final created = credential as web.PublicKeyCredential;
    final response = created.response as web.AuthenticatorAttestationResponse;
    return <String, Object?>{
      'id': created.id,
      'type': created.type,
      'response': <String, Object?>{
        'clientDataJSON': encodeBase64url(response.clientDataJSON),
        'attestationObject': encodeBase64url(response.attestationObject),
      },
    };
  }

  @override
  Future<Map<String, Object?>> getCredential(Map<String, Object?> options) async {
    final publicKey = web.PublicKeyCredentialRequestOptions(
      challenge: _bufferSource(options['challenge']),
      timeout: _timeout(options['timeout']),
      rpId: options['rpId'] as String? ?? '',
      allowCredentials: _allowedCredentials(options['allowCredentials']),
      userVerification: options['userVerification'] as String? ?? 'required',
    );
    final credential = await _call(
      () => web.window.navigator.credentials
          .get(web.CredentialRequestOptions(publicKey: publicKey))
          .toDart,
    );
    if (credential == null) {
      // 利用者が選択を取り消すと null が返る (W3C WebAuthn Level 3 の 5.1.4)。
      throw const PasskeyError(PasskeyErrorKind.cancelled, 'the credential is null');
    }
    final asserted = credential as web.PublicKeyCredential;
    final response = asserted.response as web.AuthenticatorAssertionResponse;
    return <String, Object?>{
      // サーバーは保存済みのクレデンシャル ID と照合する (base64url)。
      'id': encodeBase64url(asserted.rawId),
      'type': asserted.type,
      'response': <String, Object?>{
        'clientDataJSON': encodeBase64url(response.clientDataJSON),
        'authenticatorData': encodeBase64url(response.authenticatorData),
        'signature': encodeBase64url(response.signature),
      },
    };
  }

  /// `navigator.credentials` の呼び出しを包み、JS の例外を [PasskeyError] にする。
  static Future<web.Credential?> _call(Future<web.Credential?> Function() request) async {
    try {
      return await request();
    } catch (error) {
      throw PasskeyError(kindOf(error), error);
    }
  }

  /// JS の例外から失敗の種類を決める。
  ///
  /// 取り消しとタイムアウトは `NotAllowedError` で返る (W3C WebAuthn Level 3 の 5.1.4)。
  /// JS の例外の名前をアプリの種別にする (画面の文言の分岐に使う)。テストから確認できるように公開する。
  @visibleForTesting
  static PasskeyErrorKind kindOf(Object error) {
    switch (_errorName(error)) {
      case 'NotAllowedError':
      case 'AbortError':
        return PasskeyErrorKind.cancelled;
      case 'NotSupportedError':
        return PasskeyErrorKind.unsupported;
      default:
        return PasskeyErrorKind.failed;
    }
  }

  /// JS の例外の `name` を読む。
  ///
  /// このファイルは Web でだけコンパイルされるため、`Object` と JS の型の実行時の判定は
  /// プラットフォームで結果が変わらない。
  static String? _errorName(Object error) {
    // ignore: invalid_runtime_check_with_js_interop_types
    if (error is JSObject) {
      return error.getProperty<JSString?>('name'.toJS)?.toDart;
    }
    return null;
  }

  /// `rp` のオプションを JS の辞書にする。
  static web.PublicKeyCredentialRpEntity _relyingParty(Object? value) {
    final rp = value as Map<String, Object?>;
    return web.PublicKeyCredentialRpEntity(
      id: rp['id'] as String? ?? '',
      name: rp['name'] as String? ?? '',
    );
  }

  /// `user` のオプションを JS の辞書にする。
  ///
  /// `displayName` は必須メンバーだが、サーバーのオプションには含まれない (ADR-0004)。
  /// 管理者が使う表示名は入れず、`name` (利用者の UUID) で埋める。
  static web.PublicKeyCredentialUserEntity _userEntity(Object? value) {
    final user = value as Map<String, Object?>;
    final name = user['name'] as String? ?? '';
    return web.PublicKeyCredentialUserEntity(
      id: _bufferSource(user['id']),
      name: name,
      displayName: user['displayName'] as String? ?? name,
    );
  }

  /// `pubKeyCredParams` のオプションを JS の配列にする。
  static JSArray<web.PublicKeyCredentialParameters> _algorithms(Object? value) {
    final parameters = value is List<Object?> ? value : const <Object?>[];
    return parameters
        .cast<Map<String, Object?>>()
        .map(
          (parameter) => web.PublicKeyCredentialParameters(
            type: parameter['type'] as String? ?? 'public-key',
            alg: (parameter['alg'] as num?)?.toInt() ?? -7,
          ),
        )
        .toList()
        .toJS;
  }

  /// `authenticatorSelection` のオプションを JS の辞書にする。
  static web.AuthenticatorSelectionCriteria _authenticatorSelection(Object? value) {
    final selection = value is Map<String, Object?> ? value : const <String, Object?>{};
    // ADR-0004 の要求 (residentKey は preferred、userVerification は required) を既定にする。
    return web.AuthenticatorSelectionCriteria(
      residentKey: selection['residentKey'] as String? ?? 'preferred',
      userVerification: selection['userVerification'] as String? ?? 'required',
    );
  }

  /// `allowCredentials` のオプションを JS の配列にする。
  ///
  /// 空の配列は discoverable なパスキーを対象にする (FR-2)。
  static JSArray<web.PublicKeyCredentialDescriptor> _allowedCredentials(Object? value) {
    final descriptors = value is List<Object?> ? value : const <Object?>[];
    return descriptors
        .cast<Map<String, Object?>>()
        .map(
          (descriptor) => web.PublicKeyCredentialDescriptor(
            type: descriptor['type'] as String? ?? 'public-key',
            id: _bufferSource(descriptor['id']),
          ),
        )
        .toList()
        .toJS;
  }

  /// ミリ秒のタイムアウトにする。無い場合は 0 とし、タイムアウトを課さない
  /// (W3C WebAuthn Level 3 の 5.1.3)。
  static int _timeout(Object? value) => (value as num?)?.toInt() ?? 0;

  /// base64url の文字列を、バイト列を受け取るメンバー ([web.BufferSource]) にする。
  static web.BufferSource _bufferSource(Object? value) {
    return decodeBase64url(value as String).toJS;
  }

  /// base64url を復号する。Dart の base64 の復号はパディングを要求するため、4 の倍数に足す。
  @visibleForTesting
  static Uint8List decodeBase64url(String text) {
    final padding = (4 - text.length % 4) % 4;
    return base64Url.decode(text.padRight(text.length + padding, '='));
  }

  /// base64url に符号化する。パディングは付けない (W3C WebAuthn Level 3 の 3)。
  @visibleForTesting
  static String encodeBase64url(JSArrayBuffer buffer) {
    final encoded = base64Url.encode(buffer.toDart.asUint8List());
    return encoded.replaceAll('=', '');
  }
}

/// Web の [PasskeyClient] を返す。
PasskeyClient createPasskeyClient() => WebPasskeyClient();
