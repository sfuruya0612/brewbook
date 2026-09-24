// Canvas を使うテストはブラウザでだけ動かす (frontend:test-web)。
@TestOn('browser')
library;

import 'package:coffee_log/api/api_error.dart';
import 'package:coffee_log/api/records_api.dart';
import 'package:coffee_log/photo/image_converter_web.dart';
import 'package:coffee_log/photo/photo_uploader.dart';
import 'package:flutter_test/flutter_test.dart';

import '../support/fake_api.dart';
import '../support/fake_photo.dart';
import '../support/fake_records.dart';
import 'canvas_images.dart';

/// 写真のアップロード (FR-10、ADR-0003) のブラウザでのテスト。
///
/// 長辺 4,000 px の入力を Canvas で作り、クライアント側の変換から 3 回の呼び出し
/// (URL の発行、R2 への PUT、完了の通知) までを通しで確認する。
void main() {
  /// 写真の 3 回の呼び出しの応答を登録する。
  FakeApi photoApi() {
    return FakeApi()
      ..on(
        'POST',
        '/api/purchases/purchase-1/photo/upload-url',
        status: 200,
        body: <String, Object?>{'url': 'https://r2.example/upload', 'key': 'pending/user-1/x.jpg'},
      )
      ..on(
        'POST',
        '/api/purchases/purchase-1/photo',
        status: 200,
        body: purchaseJson(
          id: 'purchase-1',
          photoKey: 'users/user-1/purchases/purchase-1/x.jpg',
        ),
      );
  }

  test('4,000 px の写真を変換し、URL の発行と PUT と完了通知を行う', () async {
    // 長辺 4,000 px の入力を Canvas で作る (FR-10 の受け入れ基準)。
    final input = await createCanvasImage(4000, 3000, 'image/png');
    final converted = await const CanvasImageConverter().convertJpeg(input);
    // 変換後のサイズは 5 MB 以下である (ADR-0003)。
    expect(converted.size, lessThanOrEqualTo(5000000));

    final api = photoApi();
    final upload = FakeUploadClient();
    final uploader = PhotoUploader(records: RecordsApi(api.client()), httpClient: upload.client());

    final purchase = await uploader.upload(purchaseId: 'purchase-1', image: converted);

    // 変換後のサイズを申告して URL を要求する (ADR-0003)。
    expect(api.lastBody('POST', '/api/purchases/purchase-1/photo/upload-url'), <String, Object?>{
      'size': converted.size,
    });
    // R2 の S3 互換エンドポイントへ直接 PUT する (Content-Type は image/jpeg)。
    expect(upload.requests, hasLength(1));
    final request = upload.requests.single;
    expect(request.url.toString(), 'https://r2.example/upload');
    expect(request.headers['Content-Type'], 'image/jpeg');
    expect(request.bodyBytes, converted.bytes);
    // 完了を通知する (ADR-0003)。
    expect(api.lastBody('POST', '/api/purchases/purchase-1/photo'), <String, Object?>{
      'key': 'pending/user-1/x.jpg',
      'size': converted.size,
    });
    // 呼び出しは URL の発行と PUT と完了通知の 3 回である (ADR-0003)。
    expect(api.calls, <String>[
      'POST /api/purchases/purchase-1/photo/upload-url',
      'POST /api/purchases/purchase-1/photo',
    ]);
    expect(purchase.photoKey, 'users/user-1/purchases/purchase-1/x.jpg');
  });

  test('PUT が失敗すると再試行を促す表示の失敗になる', () async {
    final input = await createCanvasImage(100, 100, 'image/jpeg');
    final converted = await const CanvasImageConverter().convertJpeg(input);
    final api = photoApi();
    final upload = FakeUploadClient()..status = 403;
    final uploader = PhotoUploader(records: RecordsApi(api.client()), httpClient: upload.client());

    await expectLater(
      uploader.upload(purchaseId: 'purchase-1', image: converted),
      throwsA(isA<NetworkError>()),
    );
    // 完了は通知しない。
    expect(api.calls, <String>['POST /api/purchases/purchase-1/photo/upload-url']);
  });
}
