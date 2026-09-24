// Canvas を使うテストはブラウザでだけ動かす (frontend:test-web)。
@TestOn('browser')
library;

import 'dart:math' as math;

import 'package:coffee_log/photo/image_converter.dart';
import 'package:coffee_log/photo/image_converter_web.dart';
import 'package:flutter_test/flutter_test.dart';

import 'canvas_images.dart';

/// 写真の変換 (FR-10、ADR-0003) のブラウザでのテスト。
void main() {
  const converter = CanvasImageConverter();

  test('長辺 4,000 px の PNG と JPEG から、JPEG で長辺 2,048 px 以下の出力が得られる', () async {
    for (final format in <String>['image/png', 'image/jpeg']) {
      // 長辺 4,000 px の入力を Canvas で作る (FR-10 の受け入れ基準)。
      final input = await createCanvasImage(4000, 3000, format);
      expect(await decodeCanvasImage(input), (width: 4000, height: 3000), reason: format);

      final converted = await converter.convertJpeg(input);

      // 出力は JPEG である (FR-10)。
      expect(converted.bytes[0], 0xFF, reason: format);
      expect(converted.bytes[1], 0xD8, reason: format);
      expect(converted.bytes[2], 0xFF, reason: format);
      // 長辺は 2,048 px 以下で、縦横の比を保つ (FR-10)。
      final output = await decodeCanvasImage(converted.bytes);
      expect(math.max(output.width, output.height), lessThanOrEqualTo(maxPhotoLongSide));
      expect(output, (width: 2048, height: 1536), reason: format);
      // 変換後のサイズ (バイト) を取得できる (ADR-0003 の申告に使う)。
      expect(converted.size, converted.bytes.length);
    }
  });

  test('長辺が上限以下の入力は縮小しない', () async {
    final input = await createCanvasImage(800, 600, 'image/png');

    final converted = await converter.convertJpeg(input);

    expect(await decodeCanvasImage(converted.bytes), (width: 800, height: 600));
    expect(converted.size, lessThanOrEqualTo(5000000));
  });

  test('長辺を指定して縮小できる', () async {
    final input = await createCanvasImage(4000, 2000, 'image/jpeg');

    final converted = await converter.convertJpeg(input, maxLongSide: 1000);

    expect(await decodeCanvasImage(converted.bytes), (width: 1000, height: 500));
  });
}
