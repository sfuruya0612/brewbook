/// 入力の値の読み書き (FR-9、FR-11、FR-16)。
///
/// 日付は `YYYY-MM-DD`、時刻は `HH:MM`、抽出日時は ISO 8601 の UTC で API と受け渡しする。
/// 画面の入力は端末のローカル時刻で行い、送信の直前に UTC へ変換する (FR-11)。
library;

import 'package:flutter/widgets.dart';
import 'package:intl/intl.dart';

/// 日付だけを表す正規表現 (`YYYY-MM-DD`)。
final RegExp _dayPattern = RegExp(r'^(\d{4})-(\d{2})-(\d{2})$');

/// 時刻だけを表す正規表現 (`HH:MM`)。
final RegExp _timePattern = RegExp(r'^(\d{2}):(\d{2})$');

/// 0 以上の整数を表す正規表現 (小数と符号と指数は受け付けない)。
final RegExp _countPattern = RegExp(r'^\d+$');

/// 0 以上で小数第 1 位までの数を表す正規表現 (API の検証と同じ条件)。
final RegExp _decimalPattern = RegExp(r'^\d+(\.\d)?$');

/// 端末のタイムゾーンでの当日の日付 (購入日の既定値。FR-9)。
DateTime today() => DateTime.now();

/// 端末のタイムゾーンでの現在の日時 (抽出日時の既定値。FR-11)。
DateTime now() => DateTime.now();

/// 日付 (`YYYY-MM-DD`) にする。
String formatDay(DateTime day) {
  final year = day.year.toString().padLeft(4, '0');
  final month = day.month.toString().padLeft(2, '0');
  final dayOfMonth = day.day.toString().padLeft(2, '0');
  return '$year-$month-$dayOfMonth';
}

/// 日付 (`YYYY-MM-DD`) を読む。実在しない日付と形式の違反は null にする。
DateTime? parseDay(String text) {
  final match = _dayPattern.firstMatch(text.trim());
  if (match == null) {
    return null;
  }
  final year = int.parse(match.group(1)!);
  final month = int.parse(match.group(2)!);
  final dayOfMonth = int.parse(match.group(3)!);
  final day = DateTime(year, month, dayOfMonth);
  // DateTime は存在しない日を繰り上げるため、組み立てた値が入力と一致するかで確かめる。
  if (day.year != year || day.month != month || day.day != dayOfMonth) {
    return null;
  }
  return day;
}

/// 時刻 (`HH:MM`) を読む。読めなければ null にする。
({int hour, int minute})? parseTime(String text) {
  final match = _timePattern.firstMatch(text.trim());
  if (match == null) {
    return null;
  }
  final hour = int.parse(match.group(1)!);
  final minute = int.parse(match.group(2)!);
  if (hour > 23 || minute > 59) {
    return null;
  }
  return (hour: hour, minute: minute);
}

/// 時刻を `HH:MM` にする。
String formatTime(int hour, int minute) {
  return '${hour.toString().padLeft(2, '0')}:${minute.toString().padLeft(2, '0')}';
}

/// 端末のローカル時刻の日時を、送信用の UTC の ISO 8601 にする (FR-11)。
String toUtcIso8601(DateTime local) => local.toUtc().toIso8601String();

/// UTC の ISO 8601 の日時を、端末のローカル時刻にする (表示用。FR-11)。
DateTime parseUtcToLocal(String text) => DateTime.parse(text).toLocal();

/// 0 以上の整数を読む。読めなければ null にする (FR-9、FR-11)。
int? parseCount(String text) {
  final value = text.trim();
  if (!_countPattern.hasMatch(value)) {
    return null;
  }
  return int.tryParse(value);
}

/// 0 以上で小数第 1 位までの数を読む。読めなければ null にする (FR-11)。
double? parseDecimal(String text) {
  final value = text.trim();
  if (!_decimalPattern.hasMatch(value)) {
    return null;
  }
  return double.tryParse(value);
}

/// 数を表示用の文字列にする。整数のときは小数部を付けない。
String formatNumber(double value) {
  if (value == value.roundToDouble()) {
    return value.toStringAsFixed(0);
  }
  return value.toStringAsFixed(1);
}

/// 日付を端末の言語で表示する (FR-16)。
String displayDay(String day, Locale locale) {
  final parsed = DateTime.tryParse(day);
  if (parsed == null) {
    return day;
  }
  return DateFormat.yMd(locale.toString()).format(parsed);
}

/// UTC の日時を端末のタイムゾーンと言語で表示する (FR-11、FR-16)。
String displayTimestamp(String timestamp, Locale locale) {
  final parsed = DateTime.tryParse(timestamp);
  if (parsed == null) {
    return timestamp;
  }
  return DateFormat.yMd(locale.toString()).add_Hm().format(parsed.toLocal());
}
