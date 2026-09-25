import '../api/api_client.dart';
import '../download/file_download.dart';

/// 設定の画面が使う依存の束 (ADR-0007)。
///
/// エクスポートの取得は [ApiClient]、保存は実行環境で実装が変わる [FileDownload] が担う。
/// 実行環境に合う実装は [createFileDownload] が返し、テストは偽の実装を差し込む。
class SettingsServices {
  SettingsServices({required ApiClient apiClient, FileDownload? fileDownload})
    : _api = apiClient,
      fileDownload = fileDownload ?? createFileDownload();

  final ApiClient _api;

  /// ダウンロードの保存 (FR-14)。
  final FileDownload fileDownload;

  /// エクスポートのファイルの名前。Backend が付ける名前と同じにする (FR-14)。
  static const String exportFileName = 'coffee-log-export.json';

  /// 全記録のエクスポートを取得し、JSON のファイルとしてダウンロードする (FR-14)。
  Future<void> exportAll() async {
    final bytes = await _api.getBytes('/export');
    await fileDownload.save(DownloadedFile(fileName: exportFileName, bytes: bytes));
  }
}
