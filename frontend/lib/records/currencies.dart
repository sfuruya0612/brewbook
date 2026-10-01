/// 通貨コードのマスタ (FR-9)。
///
/// ISO 4217 はアプリが管理するデータではなく外部の規格のため、D1 のテーブルと
/// 配る API は持たず、Flutter 内の固定リストとして持つ。追加の要望があれば
/// [currencyCodes] と通貨名 (ARB の `currencyXxx`) を足す。
library;

import '../l10n/app_localizations.dart';

/// プルダウンに載せる主要な通貨 30 種。コードの昇順に並べる。
const List<String> currencyCodes = <String>[
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

/// 通貨コードの通貨名 (ARB の `currencyXxx`。CLDR の表示名に合わせる)。
///
/// マスタに無いコードは名前を付けず、コードをそのまま返す。
String currencyName(AppLocalizations l10n, String code) {
  return switch (code) {
    'AED' => l10n.currencyAed,
    'AUD' => l10n.currencyAud,
    'BRL' => l10n.currencyBrl,
    'CAD' => l10n.currencyCad,
    'CHF' => l10n.currencyChf,
    'CNY' => l10n.currencyCny,
    'CZK' => l10n.currencyCzk,
    'DKK' => l10n.currencyDkk,
    'EUR' => l10n.currencyEur,
    'GBP' => l10n.currencyGbp,
    'HKD' => l10n.currencyHkd,
    'IDR' => l10n.currencyIdr,
    'INR' => l10n.currencyInr,
    'JPY' => l10n.currencyJpy,
    'KRW' => l10n.currencyKrw,
    'MXN' => l10n.currencyMxn,
    'MYR' => l10n.currencyMyr,
    'NOK' => l10n.currencyNok,
    'NZD' => l10n.currencyNzd,
    'PHP' => l10n.currencyPhp,
    'PLN' => l10n.currencyPln,
    'SAR' => l10n.currencySar,
    'SEK' => l10n.currencySek,
    'SGD' => l10n.currencySgd,
    'THB' => l10n.currencyThb,
    'TRY' => l10n.currencyTry,
    'TWD' => l10n.currencyTwd,
    'USD' => l10n.currencyUsd,
    'VND' => l10n.currencyVnd,
    'ZAR' => l10n.currencyZar,
    _ => code,
  };
}

/// プルダウンの選択肢の表示。マスタにあるコードは「コード 通貨名」(例: `JPY 日本円`)、
/// マスタに無いコードは名前を付けずコードだけにする。
String currencyOptionLabel(AppLocalizations l10n, String code) {
  if (!currencyCodes.contains(code)) {
    return code;
  }
  return '$code ${currencyName(l10n, code)}';
}

/// プルダウンの選択肢のコード。コードの昇順に並べる。
///
/// 現在値がマスタに無いときは、そのコードを足して選べるようにする (編集の互換)。
/// 現在値がマスタにあるときは同じコードを 2 つ作らない (選択値と同じ項目が 2 つ以上
/// あると `DropdownButton` が assert で落ちるため)。
List<String> currencyOptions(String? current) {
  if (current == null || currencyCodes.contains(current)) {
    return currencyCodes;
  }
  return <String>[...currencyCodes, current]..sort();
}
