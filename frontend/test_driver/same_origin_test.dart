//! 実バックエンドの統合テスト (0017) のホスト側のドライバ。
//!
//! `flutter drive` は WebDriver のセッションの情報を環境変数で渡す。このドライバは chromedriver の
//! WebDriver のコマンド (`goog/cdp/execute`) で Chrome に仮想認証器を付けてから、integration_test の
//! ドライバを起動する (PRD の制約: パスキーを伴う統合テストは Chrome DevTools Protocol の
//! 仮想認証器で行う)。仮想認証器の設定は 0005 の Rust のハーネスと同じにする (CTAP2、内部認証器、
//! discoverable なクレデンシャル、利用者の確認に常に成功する)。
//!
//! ブラウザが開く URL は `flutter drive --web-launch-url` が決める (Rust のハーネスが Worker の
//! オリジンの `/register?token=...` を渡す)。このドライバは App の画面を操作しない。

import 'dart:convert';
import 'dart:io';

import 'package:integration_test/integration_test_driver.dart';

Future<void> main() async {
  await addVirtualAuthenticator();
  // テストが固まったときに 20 分 (既定) 待たないようにする。失敗したときも、
  // テストが記録した URL と表示の文言 (`reportData`) を残す。
  await integrationDriver(
    timeout: const Duration(minutes: 10),
    writeResponseOnFailure: true,
  );
}

/// Chrome に仮想認証器を 1 つ付ける。
Future<void> addVirtualAuthenticator() async {
  await _cdp('WebAuthn.enable', <String, Object?>{});
  await _cdp('WebAuthn.addVirtualAuthenticator', <String, Object?>{
    'options': <String, Object?>{
      'protocol': 'ctap2',
      'transport': 'internal',
      'hasResidentKey': true,
      'hasUserVerification': true,
      'isUserVerified': true,
      'automaticPresenceSimulation': true,
    },
  });
}

/// chromedriver の WebDriver のセッションで、CDP のコマンドを 1 つ実行する。
Future<void> _cdp(String command, Map<String, Object?> params) async {
  final sessionId = Platform.environment['DRIVER_SESSION_ID'];
  final sessionUri = Platform.environment['DRIVER_SESSION_URI'];
  if (sessionId == null || sessionUri == null) {
    throw StateError(
      'flutter drive must set DRIVER_SESSION_ID and DRIVER_SESSION_URI',
    );
  }
  final uri = Uri.parse(
    sessionUri,
  ).resolve('session/$sessionId/goog/cdp/execute');
  final client = HttpClient();
  try {
    final request = await client.postUrl(uri);
    request.headers.contentType = ContentType.json;
    // 本文は長さを明示して送る。chunked で送ると chromedriver が本文を読めない。
    final body = utf8.encode(
      jsonEncode(<String, Object?>{'cmd': command, 'params': params}),
    );
    request.contentLength = body.length;
    request.add(body);
    final response = await request.close();
    final text = await response.transform(utf8.decoder).join();
    if (response.statusCode != 200) {
      throw StateError(
        'the CDP command $command failed with ${response.statusCode}: $text',
      );
    }
    final decoded = jsonDecode(text);
    if (decoded is! Map<String, Object?> ||
        decoded['value'] is! Map<String, Object?>) {
      throw StateError('the CDP command $command returned no value: $text');
    }
  } finally {
    client.close();
  }
}
