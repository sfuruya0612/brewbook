import 'dart:js_interop';
import 'dart:typed_data';

import 'package:web/web.dart' as web;

import 'file_download.dart';

/// ダウンロードのファイルの MIME タイプ (FR-14 のエクスポートは JSON)。
const String downloadContentType = 'application/json';

/// Web の [FileDownload] (FR-14)。
///
/// 応答のバイト列を Blob にし、そのオブジェクト URL を指すダウンロードのリンクを作って
/// 押す。リンクは押した後に外し、オブジェクト URL は解放する。認証は同じオリジンの
/// Cookie で行われるため (ADR-0005)、ここではヘッダを扱わない。
class BrowserFileDownload implements FileDownload {
  const BrowserFileDownload();

  @override
  Future<void> save(DownloadedFile file) async {
    final blob = web.Blob(
      <JSAny>[Uint8List.fromList(file.bytes).toJS].toJS,
      web.BlobPropertyBag(type: downloadContentType),
    );
    final url = web.URL.createObjectURL(blob);
    final link = downloadLink(url, file.fileName);
    // リンクは文書に繋がっていないと押せないブラウザがある。
    web.document.body?.appendChild(link);
    link.click();
    link.remove();
    web.URL.revokeObjectURL(url);
  }

  /// ダウンロードのリンクを組み立てる。ブラウザのテストが参照する。
  static web.HTMLAnchorElement downloadLink(String url, String fileName) {
    return web.HTMLAnchorElement()
      ..href = url
      ..download = fileName;
  }
}

/// Web の [FileDownload] を返す。
FileDownload createFileDownload() => const BrowserFileDownload();
