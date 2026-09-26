import 'dart:typed_data';

import 'package:brew_book/photo/image_converter.dart';
import 'package:brew_book/photo/photo_picker.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

/// テスト用の写真の選択 (FR-10)。設定した写真を返し、呼び出しを数える。
class FakePhotoPicker implements PhotoPicker {
  FakePhotoPicker({this.photo});

  /// 返す写真。null のときは取り消しとして null を返す。
  PickedPhoto? photo;

  /// 呼ばれた回数。
  int calls = 0;

  @override
  Future<PickedPhoto?> pickPhoto() async {
    calls += 1;
    return photo;
  }
}

/// テスト用の画像の変換 (FR-10)。
///
/// Canvas を使わずに、決めたバイト列を変換の結果として返す (Canvas を使うテストは
/// `test/web/image_converter_test.dart` が担う)。
class FakeImageConverter implements ImageConverter {
  FakeImageConverter({this.bytes});

  /// 変換の結果として返すバイト列。null のときは入力をそのまま返す。
  Uint8List? bytes;

  /// 呼ばれた回数。
  int calls = 0;

  /// 直近の入力のサイズ。
  int? lastInputSize;

  @override
  Future<ConvertedImage> convertJpeg(
    Uint8List input, {
    int maxLongSide = maxPhotoLongSide,
  }) async {
    calls += 1;
    lastInputSize = input.length;
    return ConvertedImage(bytes ?? input);
  }
}

/// テスト用の写真のアップロード先 (R2 の S3 互換エンドポイント) のクライアント (FR-10)。
///
/// PUT の要求を記録し、決めた状態コードを返す。
class FakeUploadClient {
  /// 受け取った PUT の要求。
  final List<http.Request> requests = <http.Request>[];

  /// 返す状態コード。
  int status = 200;

  /// true のときは接続の失敗 (例外) にする。
  bool networkError = false;

  /// 記録する [http.Client] を組み立てる。
  http.Client client() {
    return MockClient((request) async {
      requests.add(request);
      if (networkError) {
        throw http.ClientException('the connection failed', request.url);
      }
      return http.Response('', status);
    });
  }
}
