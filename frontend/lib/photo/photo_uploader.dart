import 'package:http/http.dart' as http;

import '../api/api_error.dart';
import '../api/models.dart';
import '../api/records_api.dart';
import 'image_converter.dart';

/// 写真のアップロード (FR-10、ADR-0003)。
///
/// 変換済みの写真について、アップロード用 URL の要求、R2 への PUT、完了の通知の 3 回の
/// 呼び出しを行う (ADR-0003)。PUT は R2 の S3 互換エンドポイント (別オリジン) に直接行い、
/// `Content-Type: image/jpeg` を付ける。CORS は R2 のバケットの設定が許可する (0009)。
class PhotoUploader {
  PhotoUploader({required this.records, http.Client? httpClient})
    : _http = httpClient ?? http.Client();

  /// 記録の API (FR-10 のアップロード用 URL の発行と完了の通知に使う)。
  final RecordsApi records;

  final http.Client _http;

  /// 写真をアップロードし、紐づいた後の購入を返す。差し替えも同じ手順で行う (FR-10)。
  Future<Purchase> upload({required String purchaseId, required ConvertedImage image}) async {
    // 変換後のサイズを申告して、署名付きの PUT の URL を受け取る (ADR-0003)。
    final target = await records.requestPhotoUploadUrl(purchaseId, image.size);
    final http.Response response;
    try {
      response = await _http.put(
        Uri.parse(target.url),
        headers: const <String, String>{'Content-Type': 'image/jpeg'},
        body: image.bytes,
      );
    } on Exception catch (error) {
      // 接続の失敗は再試行を促す表示にする (ADR-0007)。
      throw NetworkError(error);
    }
    if (response.statusCode >= 400) {
      // R2 のエラーは XML で返るため、共通の型にせず再試行を促す表示にする (ADR-0007)。
      throw NetworkError('the upload failed with status ${response.statusCode}');
    }
    // アップロードの完了を通知し、購入に紐づける (FR-10)。
    return records.completePhoto(purchaseId, key: target.key, size: image.size);
  }
}
