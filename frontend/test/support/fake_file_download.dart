import 'package:coffee_log/download/file_download.dart';

/// テスト用のダウンロード。保存の呼び出しを記録し、実際のファイルは作らない。
class FakeFileDownload implements FileDownload {
  /// 保存したファイル。
  final List<DownloadedFile> saved = <DownloadedFile>[];

  /// 保存を失敗させるときの例外。null なら成功する。
  Object? failure;

  @override
  Future<void> save(DownloadedFile file) async {
    final failure = this.failure;
    if (failure != null) {
      throw failure;
    }
    saved.add(file);
  }
}
