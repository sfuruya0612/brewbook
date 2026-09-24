import 'dart:async';
import 'dart:js_interop';

import 'package:web/web.dart' as web;

import 'photo_picker.dart';

/// 写真のファイルの選択が受け付ける形式 (FR-10)。
///
/// Web で復号できる画像はブラウザに依存する。HEIC の変換は iOS の対応時に扱う (ADR-0007)。
const String acceptedPhotoTypes = 'image/jpeg,image/png,image/webp';

/// Web の写真の選択 (FR-10)。`<input type="file">` のファイルの選択を開く。
class BrowserPhotoPicker implements PhotoPicker {
  const BrowserPhotoPicker();

  @override
  Future<PickedPhoto?> pickPhoto() {
    final completer = Completer<PickedPhoto?>();
    final input = web.HTMLInputElement()
      ..type = 'file'
      ..accept = acceptedPhotoTypes;
    // 選択のダイアログを開くには、input が文書に繋がっている必要があるブラウザがある。
    web.document.body?.appendChild(input);
    input.addEventListener('change', ((web.Event event) {
      _read(completer, input);
    }).toJS);
    // 選択を取り消したときは `cancel` が届く (Chrome など)。届かないブラウザでは
    // 選ばれるまで Future が完了しない。
    input.addEventListener('cancel', ((web.Event event) {
      _finish(completer, null, input);
    }).toJS);
    input.click();
    return completer.future;
  }

  /// 選択されたファイルを読む。選ばれていなければ null で完了する。
  static void _read(Completer<PickedPhoto?> completer, web.HTMLInputElement input) {
    final file = input.files?.item(0);
    if (file == null) {
      _finish(completer, null, input);
      return;
    }
    file
        .arrayBuffer()
        .toDart
        .then((buffer) {
          _finish(
            completer,
            PickedPhoto(name: file.name, bytes: buffer.toDart.asUint8List()),
            input,
          );
        })
        .catchError((Object error) {
          if (!completer.isCompleted) {
            completer.completeError(error);
          }
          input.remove();
        });
  }

  /// Future を完了し、input を文書から外す。2 回目以降の呼び出しは無視する。
  static void _finish(Completer<PickedPhoto?> completer, PickedPhoto? photo, web.HTMLInputElement input) {
    if (!completer.isCompleted) {
      completer.complete(photo);
    }
    input.remove();
  }
}

/// Web の [PhotoPicker] を返す。
PhotoPicker createPhotoPicker() => const BrowserPhotoPicker();
