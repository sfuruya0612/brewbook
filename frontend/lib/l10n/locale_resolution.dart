import 'package:flutter/widgets.dart';

/// UI の言語を決める (FR-16)。
///
/// 端末またはブラウザの言語が日本語なら日本語、それ以外は英語とする。
/// 対応する言語は日本語と英語の 2 つだけなので、英語以外の言語は英語にする。
Locale resolveLocale(Locale? locale, Iterable<Locale> supportedLocales) {
  if (locale?.languageCode == 'ja') {
    return const Locale('ja');
  }
  return const Locale('en');
}
