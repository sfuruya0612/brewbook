import 'file_download_stub.dart'
    if (dart.library.js_interop) 'file_download_web.dart' as implementation;

/// ダウンロードするファイル (FR-14)。
class DownloadedFile {
  const DownloadedFile({required this.fileName, required this.bytes});

  /// 保存するファイルの名前。
  final String fileName;

  /// ファイルの内容 (エクスポートの JSON など)。
  final List<int> bytes;
}

/// 応答の内容をファイルとしてダウンロードする (FR-14、ADR-0007)。
///
/// Web の実装は応答を Blob にしてダウンロードのリンクを作る。iOS の実装は配布形態が
/// 決まってから足すため、このインターフェースの後ろに置く (ADR-0007)。
abstract class FileDownload {
  /// ファイルを保存する。
  Future<void> save(DownloadedFile file);
}

/// 実行環境で使える [FileDownload] を返す。
FileDownload createFileDownload() => implementation.createFileDownload();
