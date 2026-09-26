//! `RecordsApi` の応答の読み取りの単体テスト。
//!
//! 応答の形式の違反を黙って通さないことと、URL が `ApiClient` の基準に従うことを確認する。

import 'package:brew_book/api/api_client.dart';
import 'package:brew_book/api/records_api.dart';
import 'package:flutter_test/flutter_test.dart';

import '../support/fake_api.dart';

void main() {
  test('続きのカーソルが文字列でも null でもなければ形式の違反にする', () async {
    final api = FakeApi()
      ..on('GET', '/api/brews', status: 200, body: <String, Object?>{
        'brews': <Object?>[],
        // 文字列でも null でもない値は応答の形式の違反 (静かに打ち切らない)。
        'next_cursor': 42,
      });
    final records = RecordsApi(api.client());

    await expectLater(records.brews(), throwsFormatException);
  });

  test('一覧のキーが配列でなければ形式の違反にする', () async {
    final api = FakeApi()
      ..on('GET', '/api/brews', status: 200, body: <String, Object?>{'brews': 'not an array'});
    final records = RecordsApi(api.client());

    await expectLater(records.brews(), throwsFormatException);
  });

  test('写真の URL は ApiClient の基準のパスを使う', () {
    // URL の組み立てだけを確認するため、HTTP のクライアントは差し替えない。
    final api = ApiClient(basePath: '/custom');
    final records = RecordsApi(api);

    expect(records.photoUrl('purchase-1').path, '/custom/purchases/purchase-1/photo');
  });
}
