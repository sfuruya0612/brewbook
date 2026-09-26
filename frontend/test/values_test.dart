import 'package:brew_book/records/values.dart';
import 'package:flutter_test/flutter_test.dart';

/// 入力の値の変換 (FR-9、FR-11) の単体テスト。
///
/// 画面の操作からは境界の値を試しにくいため、日付と時刻と数の読み取りをここで確認する。
void main() {
  test('日付は YYYY-MM-DD の文字列と DateTime を行き来できる', () {
    expect(formatDay(DateTime(2026, 9, 1)), '2026-09-01');
    expect(parseDay('2026-09-01'), DateTime(2026, 9, 1));
  });

  test('実在しない日付と形式の違反は読み取れない', () {
    // DateTime は 2 月 30 日を 3 月 2 日に繰り上げるため、組み立てた値で確かめる。
    expect(parseDay('2026-02-30'), isNull);
    expect(parseDay('2026-13-01'), isNull);
    expect(parseDay('2026-1-1'), isNull);
    expect(parseDay('2026/09/01'), isNull);
    expect(parseDay(''), isNull);
  });

  test('時刻は HH:MM の文字列と時と分を行き来できる', () {
    expect(formatTime(9, 30), '09:30');
    expect(formatTime(23, 59), '23:59');
    expect(parseTime('09:30'), (hour: 9, minute: 30));
    expect(parseTime(' 23:59 '), (hour: 23, minute: 59));
  });

  test('時刻の範囲外と形式の違反は読み取れない', () {
    expect(parseTime('24:00'), isNull);
    expect(parseTime('09:60'), isNull);
    expect(parseTime('9:30'), isNull);
    expect(parseTime('09:30:00'), isNull);
  });

  test('ローカルの日時は UTC の ISO 8601 になる (FR-11)', () {
    final local = DateTime(2026, 9, 1, 9, 30);
    final iso = toUtcIso8601(local);

    expect(iso, endsWith('Z'));
    expect(DateTime.parse(iso).toLocal(), local);
    expect(parseUtcToLocal(iso), local);
  });

  test('0 以上の整数だけを読み取る (FR-9)', () {
    expect(parseCount('0'), 0);
    expect(parseCount('150'), 150);
    expect(parseCount(' 200 '), 200);
    expect(parseCount('-1'), isNull);
    expect(parseCount('1.5'), isNull);
    expect(parseCount('abc'), isNull);
    expect(parseCount(''), isNull);
  });

  test('0 以上で小数第 1 位までの数だけを読み取る (FR-11)', () {
    expect(parseDecimal('0'), 0.0);
    expect(parseDecimal('15'), 15.0);
    expect(parseDecimal('15.5'), 15.5);
    expect(parseDecimal('-0.5'), isNull);
    expect(parseDecimal('15.55'), isNull);
    expect(parseDecimal('1e3'), isNull);
  });

  test('数の表示は整数のとき小数部を付けない', () {
    expect(formatNumber(15.0), '15');
    expect(formatNumber(15.5), '15.5');
  });
}
