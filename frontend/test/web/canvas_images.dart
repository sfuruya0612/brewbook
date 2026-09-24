/// Canvas を使うテストの補助 (FR-10)。ブラウザでだけ動かす。
library;

import 'dart:async';
import 'dart:js_interop';
import 'dart:typed_data';

import 'package:web/web.dart' as web;

/// Canvas に描いた画像を、[format] (`image/png` か `image/jpeg`) のバイト列にする。
///
/// 長辺 4,000 px の入力をテストで作るために使う (FR-10 の受け入れ基準)。
Future<Uint8List> createCanvasImage(int width, int height, String format) async {
  final canvas = web.HTMLCanvasElement()
    ..width = width
    ..height = height;
  final context = canvas.getContext('2d') as web.CanvasRenderingContext2D;
  context
    ..fillStyle = '#3366cc'.toJS
    ..fillRect(0, 0, width.toDouble(), height.toDouble());
  final completer = Completer<web.Blob>();
  canvas.toBlob(((web.Blob blob) => completer.complete(blob)).toJS, format);
  final blob = await completer.future;
  final buffer = await blob.arrayBuffer().toDart;
  return buffer.toDart.asUint8List();
}

/// 画像のバイト列を復号し、px の大きさを返す。
Future<({int width, int height})> decodeCanvasImage(Uint8List bytes) async {
  final url = web.URL.createObjectURL(web.Blob(<JSAny>[bytes.toJS].toJS));
  final image = web.HTMLImageElement()..src = url;
  try {
    await image.decode().toDart;
  } finally {
    web.URL.revokeObjectURL(url);
  }
  return (width: image.naturalWidth, height: image.naturalHeight);
}
