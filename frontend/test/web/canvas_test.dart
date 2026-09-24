// Canvas を使うテストはブラウザでだけ動かす (frontend:test-web)。
@TestOn('browser')
library;

import 'dart:ui' as ui;

import 'package:flutter_test/flutter_test.dart';

void main() {
  test('Canvas に描いた画像の大きさを取得できる', () async {
    final recorder = ui.PictureRecorder();
    final canvas = ui.Canvas(recorder);
    canvas.drawRect(
      const ui.Rect.fromLTWH(0, 0, 4, 4),
      ui.Paint()..color = const ui.Color(0xFF000000),
    );
    final image = await recorder.endRecording().toImage(4, 4);
    expect(image.width, 4);
    expect(image.height, 4);
    image.dispose();
  });
}
