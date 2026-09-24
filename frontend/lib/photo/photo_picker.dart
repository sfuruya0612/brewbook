import 'dart:typed_data';

import 'photo_picker_stub.dart'
    if (dart.library.js_interop) 'photo_picker_web.dart' as implementation;

/// 端末から選んだ写真 (FR-10)。
class PickedPhoto {
  const PickedPhoto({required this.name, required this.bytes});

  /// ファイルの名前。
  final String name;

  /// ファイルの内容 (JPEG、PNG、WebP など)。
  final Uint8List bytes;
}

/// 写真のファイルの選択 (FR-10、ADR-0003)。
///
/// Web の実装は `<input type="file">` のファイルの選択を開く。iOS の実装は配布形態が
/// 決まってから足すため、このインターフェースの後ろに置く (ADR-0007)。
abstract class PhotoPicker {
  /// 写真を選ばせる。取り消したときは null を返す。
  Future<PickedPhoto?> pickPhoto();
}

/// 実行環境で使える [PhotoPicker] を返す。
PhotoPicker createPhotoPicker() => implementation.createPhotoPicker();
