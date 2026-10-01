import 'dart:typed_data';

import 'image_converter_stub.dart'
    if (dart.library.js_interop) 'image_converter_web.dart' as implementation;

/// 写真の長辺の上限 (px。FR-10)。
const int maxPhotoLongSide = 2048;

/// 写真のサイズの上限 (バイト。FR-10 の申告サイズと同じ値)。
///
/// 変換の結果がこれを超えるときは、推測 (FR-19) の呼び出しを行わない。
const int maxPhotoBytes = 5000000;

/// 変換した後の写真 (FR-10)。
class ConvertedImage {
  const ConvertedImage(this.bytes);

  /// JPEG の内容。
  final Uint8List bytes;

  /// 変換後のサイズ (バイト)。アップロードの前にサーバーへ申告する (ADR-0003)。
  int get size => bytes.length;

  @override
  String toString() => 'ConvertedImage(${bytes.length} bytes)';
}

/// 写真の JPEG への変換と長辺の縮小 (FR-10、ADR-0003)。
///
/// 変換はクライアントで行い、サーバー側では行わない (PRD のやらないこと)。
/// Web の実装は Canvas API、iOS の実装はネイティブの画像処理を使う (ADR-0007)。
abstract class ImageConverter {
  /// 端末が復号できる画像を JPEG に変換し、長辺を [maxLongSide] px 以下に縮小する。
  Future<ConvertedImage> convertJpeg(Uint8List bytes, {int maxLongSide});
}

/// 実行環境で使える [ImageConverter] を返す。
ImageConverter createImageConverter() => implementation.createImageConverter();
