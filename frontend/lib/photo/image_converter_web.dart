import 'dart:async';
import 'dart:js_interop';
import 'dart:math' as math;
import 'dart:typed_data';

import 'package:web/web.dart' as web;

import 'image_converter.dart';

/// JPEG の符号化の品質。見た目を保ちながら、5 MB の上限に収まるようにする (FR-10)。
const double jpegQuality = 0.92;

/// Web の画像の変換 (FR-10、ADR-0003)。
///
/// ブラウザの Canvas API で描き直し、JPEG として符号化する。`dart:ui` の画像の符号化は
/// PNG しか出力できないため、JPEG はブラウザの符号化を使う (ADR-0007)。
class CanvasImageConverter implements ImageConverter {
  const CanvasImageConverter();

  @override
  Future<ConvertedImage> convertJpeg(
    Uint8List bytes, {
    int maxLongSide = maxPhotoLongSide,
  }) async {
    final source = await _decode(bytes);
    // 長辺が上限を超えるときだけ縮小する。縦横の比は変えない。
    final longSide = math.max(source.naturalWidth, source.naturalHeight);
    final scale = longSide > maxLongSide ? maxLongSide / longSide : 1.0;
    final width = math.max(1, (source.naturalWidth * scale).round());
    final height = math.max(1, (source.naturalHeight * scale).round());
    final canvas = web.HTMLCanvasElement()
      ..width = width
      ..height = height;
    final context = canvas.getContext('2d') as web.CanvasRenderingContext2D;
    context.drawImage(
      source,
      0,
      0,
      source.naturalWidth.toDouble(),
      source.naturalHeight.toDouble(),
      0,
      0,
      width.toDouble(),
      height.toDouble(),
    );
    final blob = await _encodeJpeg(canvas);
    final buffer = await blob.arrayBuffer().toDart;
    return ConvertedImage(buffer.toDart.asUint8List());
  }

  /// 入力のバイト列を画像として復号する (PNG、JPEG、WebP などブラウザが復号できる形式)。
  ///
  /// `createImageBitmap` は package:web に無いため、`<img>` の復号を使う。ローカルの
  /// ファイルから作ったオブジェクト URL は同一オリジンになり、Canvas は汚染されない。
  static Future<web.HTMLImageElement> _decode(Uint8List bytes) async {
    final url = web.URL.createObjectURL(web.Blob(<JSAny>[bytes.toJS].toJS));
    final image = web.HTMLImageElement()..src = url;
    try {
      await image.decode().toDart;
    } finally {
      web.URL.revokeObjectURL(url);
    }
    return image;
  }

  /// Canvas の内容を JPEG の Blob にする。`toBlob` は callback 方式のため Future にする。
  ///
  /// `toBlob` の callback は変換に失敗すると null で呼ばれる (HTML の仕様)。その場合は
  /// 完了しないまま待たず、失敗として返す。
  static Future<web.Blob> _encodeJpeg(web.HTMLCanvasElement canvas) {
    final completer = Completer<web.Blob>();
    canvas.toBlob(
      ((web.Blob? blob) => blob == null
              ? completer.completeError(
                  StateError('the canvas could not encode the image as JPEG'),
                )
              : completer.complete(blob))
          .toJS,
      'image/jpeg',
      jpegQuality.toJS,
    );
    return completer.future;
  }
}

/// Web の [ImageConverter] を返す。
ImageConverter createImageConverter() => const CanvasImageConverter();
