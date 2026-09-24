import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;

import '../api/api_client.dart';
import '../api/records_api.dart';
import '../photo/image_converter.dart';
import '../photo/photo_picker.dart';
import '../photo/photo_uploader.dart';

/// 記録の画面が使う依存の束 (ADR-0007)。
///
/// API の呼び出しと、実行環境で実装が変わる写真の選択と変換を、この 1 つの型で画面へ配る。
/// 実行環境に合う実装は [createPhotoPicker] と [createImageConverter] が返し、テストは偽の
/// 実装を差し込む。
class RecordServices {
  RecordServices({
    required ApiClient apiClient,
    PhotoPicker? photoPicker,
    ImageConverter? imageConverter,
    http.Client? photoUploadClient,
  }) : this._(
         records: RecordsApi(apiClient),
         picker: photoPicker ?? createPhotoPicker(),
         converter: imageConverter ?? createImageConverter(),
         photoUploadClient: photoUploadClient,
       );

  RecordServices._({
    required RecordsApi records,
    required this.picker,
    required this.converter,
    http.Client? photoUploadClient,
  }) : records = records,
       uploader = PhotoUploader(records: records, httpClient: photoUploadClient);

  /// 記録の API (FR-6 から FR-13)。
  final RecordsApi records;

  /// 写真のファイルの選択 (FR-10)。
  final PhotoPicker picker;

  /// 写真の JPEG への変換と縮小 (FR-10)。
  final ImageConverter converter;

  /// 写真のアップロード (FR-10)。
  final PhotoUploader uploader;

  /// 記録を変更したことの通知。一覧はこれを受けて先頭から読み直す。
  ///
  /// 登録、更新、アーカイブ、写真の操作の後に [markRecordsChanged] を呼ぶ。
  final ValueNotifier<int> revision = ValueNotifier<int>(0);

  /// 記録を変更したことを一覧に知らせる。
  void markRecordsChanged() => revision.value = revision.value + 1;

  /// 通知を解放する。
  void dispose() => revision.dispose();
}
