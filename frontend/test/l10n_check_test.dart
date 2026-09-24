// FR-16 の静的検査。ファイルを読むため VM でだけ動かす。
@TestOn('vm')
library;

import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

/// 日本語 UI の項目名の訳語 (PRD の FR-16)。
const Map<String, String> _translations = <String, String>{
  'producer': '生産者',
  'origin': '生産国',
  'region': '地域',
  'process': '精製方法',
  'variety': '品種',
  'roast': '焙煎度',
  'roastDate': '焙煎日',
  'flavorNotes': 'フレーバーノート',
};

/// Text などに文字列リテラルを直接渡していないことを調べる正規表現 (FR-16)。
final List<RegExp> _literalPatterns = <RegExp>[
  // Text('...')。SnackBar の content に渡す Text('...') もここで見つかる。
  RegExp(r"""\bText\(\s*['"]"""),
  // TextSpan(text: '...')。引数が複数行になることがあるため改行を跨ぐ。
  RegExp(r"""\bTextSpan\([^)]*?\btext:\s*['"]""", dotAll: true),
  // InputDecoration の labelText、hintText、helperText、errorText。
  RegExp(r"""\b(?:labelText|hintText|helperText|errorText):\s*['"]"""),
  // Tooltip(message: '...')。
  RegExp(r"""\bTooltip\(\s*message:\s*['"]""", dotAll: true),
  // SnackBar(content: '...')。
  RegExp(r"""\bSnackBar\(\s*content:\s*['"]""", dotAll: true),
  // AppBar(title: '...')。
  RegExp(r"""\bAppBar\(\s*title:\s*['"]""", dotAll: true),
];

void main() {
  test('日本語の ARB に 8 つの訳語がある', () async {
    final arb = jsonDecode(await File('lib/l10n/app_ja.arb').readAsString());
    expect(arb, isA<Map<String, Object?>>());
    final messages = arb as Map<String, Object?>;
    _translations.forEach((String key, String value) {
      expect(messages[key], value, reason: 'app_ja.arb の $key');
    });
  });

  test('lib の UI コードに文言を直書きしていない', () async {
    final violations = <String>[];
    final files = Directory('lib')
        .listSync(recursive: true)
        .whereType<File>()
        .where((file) => file.path.endsWith('.dart'));
    for (final file in files) {
      // 生成された翻訳は ARB の値そのものなので対象外とする (FR-16)。
      if (file.path.contains('lib/l10n/app_localizations')) {
        continue;
      }
      final source = _stripComments(await file.readAsString());
      for (final pattern in _literalPatterns) {
        for (final match in pattern.allMatches(source)) {
          final literal = _literalAfter(source, match.end);
          // URL は検査の対象外とする (FR-16)。
          if (literal.startsWith('http://') ||
              literal.startsWith('https://') ||
              literal.startsWith('/')) {
            continue;
          }
          violations.add('${file.path}: ${match.group(0)}');
        }
      }
    }
    expect(violations, isEmpty, reason: '文言は ARB から取る (FR-16)');
  });
}

/// コメントを除く。コメントの中の例は検査の対象ではない。
String _stripComments(String source) {
  return source
      .replaceAll(RegExp(r'/\*.*?\*/', dotAll: true), '')
      .replaceAll(RegExp(r'//[^\n]*'), '');
}

/// 引用符の直後から、閉じる引用符までの文字列を取る。
String _literalAfter(String source, int start) {
  final match = RegExp(r"""^[^'"]*""").firstMatch(source.substring(start));
  return match?.group(0) ?? '';
}
