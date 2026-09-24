import 'photo_picker.dart';

/// Web 以外の [PhotoPicker]。iOS の対応は配布形態が決まってから行う (ADR-0007)。
///
/// 画面は写真を選ぶときだけ選択を呼ぶため、この実装でも画面の組み立ては壊れない。
PhotoPicker createPhotoPicker() => const UnsupportedPhotoPicker();

/// Web 以外の実装。選択を呼ばれたら失敗させる。
class UnsupportedPhotoPicker implements PhotoPicker {
  const UnsupportedPhotoPicker();

  @override
  Future<PickedPhoto?> pickPhoto() {
    throw UnsupportedError('the photo picker is not implemented on this platform');
  }
}
