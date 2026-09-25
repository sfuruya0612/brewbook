import 'file_download.dart';

/// Web 以外の [FileDownload]。iOS の対応は配布形態が決まってから行う (ADR-0007)。
///
/// 画面はエクスポートを押したときだけ保存を呼ぶため、この実装でも画面の組み立ては壊れない。
FileDownload createFileDownload() => const UnsupportedFileDownload();

/// Web 以外の実装。保存を呼ばれたら失敗させる。
class UnsupportedFileDownload implements FileDownload {
  const UnsupportedFileDownload();

  @override
  Future<void> save(DownloadedFile file) {
    throw UnsupportedError('the file download is not implemented on this platform');
  }
}
