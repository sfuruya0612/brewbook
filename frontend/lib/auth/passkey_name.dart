/// パスキーの名前の検証 (FR-1、FR-3)。
///
/// 名前は前後の空白を除いて 1 文字以上 50 文字以下とする (ADR-0004)。
/// 文字数は Unicode のスカラー値で数える (Backend の `validate_passkey_name` と同じ。
/// Dart の `String.length` は UTF-16 の符号単位のため、サロゲートペアを含む名前でずれる)。
library;

import '../l10n/app_localizations.dart';

/// 名前を検証する。正しければ null を返し、正しくなければ画面に出す文言を返す。
String? validatePasskeyName(String value, AppLocalizations l10n) {
  final name = value.trim();
  if (name.isEmpty || name.runes.length > 50) {
    return l10n.passkeyNameError;
  }
  return null;
}

/// 入力の名前を、API に送る形 (前後の空白を除いた名前) にする。
String passkeyNameForRequest(String value) => value.trim();
