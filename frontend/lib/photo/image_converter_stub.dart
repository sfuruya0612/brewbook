import 'dart:typed_data';

import 'image_converter.dart';

/// Web 以外の [ImageConverter]。iOS の対応は配布形態が決まってから行う (ADR-0007)。
///
/// 画面は写真を選ぶときだけ変換を呼ぶため、この実装でも画面の組み立ては壊れない。
ImageConverter createImageConverter() => const UnsupportedImageConverter();

/// Web 以外の実装。変換を呼ばれたら失敗させる。
class UnsupportedImageConverter implements ImageConverter {
  const UnsupportedImageConverter();

  @override
  Future<ConvertedImage> convertJpeg(Uint8List bytes, {int maxLongSide = maxPhotoLongSide}) {
    throw UnsupportedError('the image converter is not implemented on this platform');
  }
}
