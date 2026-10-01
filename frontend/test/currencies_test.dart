import 'package:brew_book/l10n/app_localizations.dart';
import 'package:brew_book/records/currencies.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

/// 設計判断で確定した主要な通貨 30 種 (コードの昇順)。
const List<String> expectedCurrencyCodes = <String>[
  'AED',
  'AUD',
  'BRL',
  'CAD',
  'CHF',
  'CNY',
  'CZK',
  'DKK',
  'EUR',
  'GBP',
  'HKD',
  'IDR',
  'INR',
  'JPY',
  'KRW',
  'MXN',
  'MYR',
  'NOK',
  'NZD',
  'PHP',
  'PLN',
  'SAR',
  'SEK',
  'SGD',
  'THB',
  'TRY',
  'TWD',
  'USD',
  'VND',
  'ZAR',
];

/// 設計判断で確定した 30 種の通貨名 (CLDR の表示名。コードの昇順)。
const Map<String, ({String ja, String en})> expectedCurrencyNames =
    <String, ({String ja, String en})>{
  'AED': (ja: 'UAE ディルハム', en: 'UAE Dirham'),
  'AUD': (ja: '豪ドル', en: 'Australian Dollar'),
  'BRL': (ja: 'ブラジル レアル', en: 'Brazilian Real'),
  'CAD': (ja: 'カナダ ドル', en: 'Canadian Dollar'),
  'CHF': (ja: 'スイス フラン', en: 'Swiss Franc'),
  'CNY': (ja: '中国人民元', en: 'Chinese Yuan'),
  'CZK': (ja: 'チェコ コルナ', en: 'Czech Koruna'),
  'DKK': (ja: 'デンマーク クローネ', en: 'Danish Krone'),
  'EUR': (ja: 'ユーロ', en: 'Euro'),
  'GBP': (ja: '英ポンド', en: 'British Pound'),
  'HKD': (ja: '香港ドル', en: 'Hong Kong Dollar'),
  'IDR': (ja: 'インドネシア ルピア', en: 'Indonesian Rupiah'),
  'INR': (ja: 'インド ルピー', en: 'Indian Rupee'),
  'JPY': (ja: '日本円', en: 'Japanese Yen'),
  'KRW': (ja: '韓国ウォン', en: 'South Korean Won'),
  'MXN': (ja: 'メキシコ ペソ', en: 'Mexican Peso'),
  'MYR': (ja: 'マレーシア リンギット', en: 'Malaysian Ringgit'),
  'NOK': (ja: 'ノルウェー クローネ', en: 'Norwegian Krone'),
  'NZD': (ja: 'ニュージーランド ドル', en: 'New Zealand Dollar'),
  'PHP': (ja: 'フィリピン ペソ', en: 'Philippine Peso'),
  'PLN': (ja: 'ポーランド ズウォティ', en: 'Polish Zloty'),
  'SAR': (ja: 'サウジアラビア リヤル', en: 'Saudi Riyal'),
  'SEK': (ja: 'スウェーデン クローナ', en: 'Swedish Krona'),
  'SGD': (ja: 'シンガポール ドル', en: 'Singapore Dollar'),
  'THB': (ja: 'タイ バーツ', en: 'Thai Baht'),
  'TRY': (ja: 'トルコ リラ', en: 'Turkish Lira'),
  'TWD': (ja: '新台湾ドル', en: 'New Taiwan Dollar'),
  'USD': (ja: '米ドル', en: 'US Dollar'),
  'VND': (ja: 'ベトナム ドン', en: 'Vietnamese Dong'),
  'ZAR': (ja: '南アフリカ ランド', en: 'South African Rand'),
};

/// ISO 4217 の 3 文字の英大文字の形。
final RegExp _codePattern = RegExp(r'^[A-Z]{3}$');

void main() {
  test('通貨コードは設計判断の 30 種と一致し、形と順序と重複に問題が無い', () {
    expect(currencyCodes, expectedCurrencyCodes);
    expect(currencyCodes.length, 30);
    for (final String code in currencyCodes) {
      expect(_codePattern.hasMatch(code), isTrue, reason: '$code は ISO 4217 の形ではない');
    }
    expect(currencyCodes.toSet().length, currencyCodes.length, reason: '重複がある');
    expect(currencyCodes, orderedEquals(<String>[...currencyCodes]..sort()), reason: '昇順ではない');
    expect(currencyCodes, contains('JPY'));
  });

  test('30 種すべての通貨名が日本語と英語の ARB にあり、CLDR の表示名と一致する', () async {
    final AppLocalizations ja = await AppLocalizations.delegate.load(const Locale('ja'));
    final AppLocalizations en = await AppLocalizations.delegate.load(const Locale('en'));
    expect(
      expectedCurrencyNames.keys.toList(),
      currencyCodes,
      reason: '通貨名の表の対象がマスタと一致しない',
    );
    for (final String code in currencyCodes) {
      final ({String ja, String en}) expected = expectedCurrencyNames[code]!;
      // 名前が空でないことと、コードと異なることは、期待値の表が固定している。
      // 英語の ARB からキーが抜けた場合も、テンプレート (日本語) の値と一致しなくなる。
      expect(currencyName(ja, code), expected.ja, reason: '$code の日本語名');
      expect(currencyName(en, code), expected.en, reason: '$code の英語名');
    }
    // マスタに無いコードは名前を付けず、コードをそのまま返す。
    expect(currencyName(ja, 'ETB'), 'ETB');
    expect(currencyName(en, 'ETB'), 'ETB');
  });

  test('選択肢の表示は、マスタにあるコードは「コード 通貨名」、無いコードはコードだけ', () async {
    final AppLocalizations ja = await AppLocalizations.delegate.load(const Locale('ja'));
    final AppLocalizations en = await AppLocalizations.delegate.load(const Locale('en'));
    expect(currencyOptionLabel(ja, 'JPY'), 'JPY 日本円');
    expect(currencyOptionLabel(en, 'JPY'), 'JPY Japanese Yen');
    // マスタに無いコードは名前を付けず、コードをそのまま返す。
    expect(currencyName(ja, 'ETB'), 'ETB');
    expect(currencyOptionLabel(ja, 'ETB'), 'ETB');
    expect(currencyOptionLabel(en, 'ETB'), 'ETB');
  });

  test('選択肢はマスタの 30 種で、現在値がマスタに無いときだけ足す', () {
    expect(currencyOptions(null), currencyCodes);
    expect(currencyOptions('JPY'), currencyCodes);
    final List<String> options = currencyOptions('ETB');
    expect(options, <String>[...currencyCodes, 'ETB']..sort());
    expect(options.length, 31);
    expect(options.where((String code) => code == 'ETB').length, 1);
  });
}
