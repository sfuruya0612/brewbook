//! エクスポートのダウンロード (FR-14) の、ブラウザで動く部分のテスト。
//!
//! `URL.createObjectURL` を差し替えて、保存の経路が作る Blob の中身と、ダウンロードのリンクの
//! ファイル名を Chrome 上で確認する (`mise run frontend:test-web`)。実際のダウンロードの完了は
//! ブラウザの設定に依存するため、ここでは確認しない。

@TestOn('browser')
library;

import 'dart:convert';
import 'dart:js_interop';
import 'dart:js_interop_unsafe';

import 'package:coffee_log/download/file_download.dart';
import 'package:coffee_log/download/file_download_web.dart';
import 'package:coffee_log/settings/settings_services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:web/web.dart' as web;

import '../support/fake_api.dart';

/// `URL` のオブジェクト (静的メソッドの差し替えに使う)。
@JS('URL')
external JSObject get _url;

/// `URL.createObjectURL` と `revokeObjectURL` を差し替え、作られた Blob と解放を記録する。
///
/// ブラウザの API はダウンロードの完了を通知しないため、Blob の受け渡しを観測点にする。
({JSObject? Function() created, List<String> revoked}) hookObjectUrl() {
  final originalCreate = _url.getProperty<JSFunction?>('createObjectURL'.toJS);
  final originalRevoke = _url.getProperty<JSFunction?>('revokeObjectURL'.toJS);
  JSObject? created;
  final revoked = <String>[];
  _url.setProperty(
    'createObjectURL'.toJS,
    ((JSObject blob) {
      created = blob;
      return 'blob:coffee-log-export'.toJS;
    }).toJS,
  );
  _url.setProperty(
    'revokeObjectURL'.toJS,
    ((JSString url) {
      revoked.add(url.toDart);
    }).toJS,
  );
  addTearDown(() {
    if (originalCreate != null) {
      _url.setProperty('createObjectURL'.toJS, originalCreate);
    }
    if (originalRevoke != null) {
      _url.setProperty('revokeObjectURL'.toJS, originalRevoke);
    }
  });
  return (created: () => created, revoked: revoked);
}

void main() {
  test('ダウンロードのリンクは Blob の URL とファイル名を持つ', () {
    final link = BrowserFileDownload.downloadLink(
      'blob:coffee-log-export',
      'coffee-log-export.json',
    );
    expect(link.href, 'blob:coffee-log-export');
    expect(link.download, 'coffee-log-export.json');
  });

  test('JSON を Blob にしてダウンロードを開始し、オブジェクト URL を解放する', () async {
    final hook = hookObjectUrl();
    const body = '{"shops":[],"brews":[]}';
    await const BrowserFileDownload().save(
      DownloadedFile(
        fileName: 'coffee-log-export.json',
        bytes: utf8.encode(body),
      ),
    );

    // 応答の JSON がそのまま Blob になる (FR-14)。
    final blob = hook.created()! as web.Blob;
    expect(blob.type, 'application/json');
    final buffer = await blob.arrayBuffer().toDart;
    expect(utf8.decode(buffer.toDart.asUint8List()), body);
    // ダウンロードのリンクが作ったオブジェクト URL は解放する。
    expect(hook.revoked, <String>['blob:coffee-log-export']);
  });

  test('エクスポートの応答の JSON がブラウザのダウンロードになる', () async {
    final hook = hookObjectUrl();
    final exportBody = <String, Object?>{
      'shops': <Object?>[],
      'products': <Object?>[],
      'flavor_tags': <Object?>[],
      'product_flavor_tags': <Object?>[],
      'purchases': <Object?>[],
      'brews': <Object?>[],
    };
    final api = FakeApi()
      ..on('GET', '/api/export', status: 200, body: exportBody);
    final services = SettingsServices(
      apiClient: api.client(),
      fileDownload: const BrowserFileDownload(),
    );

    await services.exportAll();

    // エクスポートの経路を呼び、応答の JSON をそのまま Blob にする (FR-14)。
    expect(api.calls, <String>['GET /api/export']);
    final blob = hook.created()! as web.Blob;
    expect(blob.type, 'application/json');
    final buffer = await blob.arrayBuffer().toDart;
    expect(utf8.decode(buffer.toDart.asUint8List()), jsonEncode(exportBody));
    expect(hook.revoked, <String>['blob:coffee-log-export']);
  });
}
