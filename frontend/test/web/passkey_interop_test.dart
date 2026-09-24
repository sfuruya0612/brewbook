//! Web のパスキーの実装の、ブラウザで動く部分の単体テスト。
//!
//! `navigator.credentials` の呼び出し自体は仮想認証器を使う 0017 の統合テストが担う。
//! ここでは JS との境界で間違えやすい変換 (base64url のパディング、例外の種別) を
//! Chrome 上で確認する (`mise run frontend:test-web`)。

@TestOn('browser')
library;

import 'dart:js_interop';
import 'dart:typed_data';

import 'package:coffee_log/auth/passkey_client_web.dart';
import 'package:coffee_log/auth/passkey_error.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test('base64url の復号はパディングを補う', () {
    // 長さが 4 の倍数でない値も復号できる (W3C WebAuthn Level 3 の 3 はパディングを付けない)。
    expect(WebPasskeyClient.decodeBase64url('AQID'), Uint8List.fromList(<int>[1, 2, 3]));
    expect(WebPasskeyClient.decodeBase64url('AQI'), Uint8List.fromList(<int>[1, 2]));
    expect(WebPasskeyClient.decodeBase64url('AQ'), Uint8List.fromList(<int>[1]));
  });

  test('base64url の符号化はパディングを付けない', () {
    expect(
      WebPasskeyClient.encodeBase64url(Uint8List.fromList(<int>[1, 2, 3]).buffer.toJS),
      'AQID',
    );
    expect(
      WebPasskeyClient.encodeBase64url(Uint8List.fromList(<int>[1, 2]).buffer.toJS),
      'AQI',
    );
    expect(
      WebPasskeyClient.encodeBase64url(Uint8List.fromList(<int>[1]).buffer.toJS),
      'AQ',
    );
  });

  test('JS の例外の名前をアプリの種別にする', () {
    expect(WebPasskeyClient.kindOf(<String, Object?>{'name': 'NotAllowedError'}.jsify()!),
        PasskeyErrorKind.cancelled);
    expect(WebPasskeyClient.kindOf(<String, Object?>{'name': 'AbortError'}.jsify()!),
        PasskeyErrorKind.cancelled);
    expect(WebPasskeyClient.kindOf(<String, Object?>{'name': 'NotSupportedError'}.jsify()!),
        PasskeyErrorKind.unsupported);
    expect(WebPasskeyClient.kindOf(<String, Object?>{'name': 'SecurityError'}.jsify()!),
        PasskeyErrorKind.failed);
    // JS のオブジェクトでない値も安全に扱う。
    expect(WebPasskeyClient.kindOf('not a JS object'), PasskeyErrorKind.failed);
  });
}
