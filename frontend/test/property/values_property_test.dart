// 入力の値の変換の PBT (issue 0027、ADR-0013)。
//
// 生成する入力の定義域は values.dart の仕様に合わせる。
//
// - 日付: 年 0000 から 9999、月 1 から 12、日 1 から 31 の部品を生成する。
//   実在しない日付 (2 月 30 日、4 月 31 日、平年の 2 月 29 日など) は parseDay が null を返す。
// - 時刻: 時 0 から 23 と分 0 から 59。
// - 整数: 0 以上 2^53 以下。kiri_check の整数生成器が一様に生成できるのは 2^32-1 までで、
//   上位の桁は代表する定数 (2^32、2^40、2^48、2^53) で確認する。
// - 小数: 小数第 1 位までで、一様に生成できるのは (2^32-1)/10 = 429496729.5 まで。
//   上位の桁は代表する定数 (10^10、10^12、10^14、10^20、10^21 の直前の
//   9.999999999999999e20) で確認する (10^21 以上は formatNumber が指数表記になる)。
//
// 反例が出たときは、失敗の出力にある seed の値を forAll の seed に指定して再現する。
import 'package:brew_book/records/values.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:kiri_check/kiri_check.dart';

void main() {
  property('実在する日付は YYYY-MM-DD と DateTime を行き来でき、実在しない日付は読み取れない', () {
    forAll(
      combine3(
        integer(min: 0, max: 9999),
        integer(min: 1, max: 12),
        integer(min: 1, max: 31),
      ),
      (input) {
        final (year, month, dayOfMonth) = input;
        final text = '${year.toString().padLeft(4, '0')}-'
            '${month.toString().padLeft(2, '0')}-'
            '${dayOfMonth.toString().padLeft(2, '0')}';
        final value = DateTime(year, month, dayOfMonth);
        final isReal = value.year == year &&
            value.month == month &&
            value.day == dayOfMonth;

        if (isReal) {
          expect(parseDay(formatDay(value)), value);
        } else {
          expect(parseDay(text), isNull);
        }
      },
    );
  });

  property('形式の違反は読み取れない', () {
    forAll(
      combine3(
        integer(min: 0, max: 4),
        integer(min: 1, max: 12),
        integer(min: 1, max: 28),
      ),
      (input) {
        final (kind, month, dayOfMonth) = input;
        final paddedMonth = month.toString().padLeft(2, '0');
        final paddedDay = dayOfMonth.toString().padLeft(2, '0');
        final text = switch (kind) {
          0 => '$month-$dayOfMonth-2026',
          1 => '2026/$paddedMonth/$paddedDay',
          2 => '2026-${(month % 9) + 1}-${(dayOfMonth % 9) + 1}',
          3 => '2026-$paddedMonth-${paddedDay}T00:00',
          _ => '  ',
        };

        expect(parseDay(text), isNull);
      },
    );
  });

  property('時刻は HH:MM と時と分を行き来できる', () {
    forAll(
      combine2(integer(min: 0, max: 23), integer(min: 0, max: 59)),
      (input) {
        final (hour, minute) = input;

        expect(
          parseTime(formatTime(hour, minute)),
          (hour: hour, minute: minute),
        );
      },
    );
  });

  property('0 以上 2^53 以下の整数は表示と読み取りを行き来できる', () {
    forAll(
      oneOf([
        integer(min: 0, max: 4294967295),
        constant(4294967296),
        constant(1099511627776),
        constant(281474976710656),
        constant(9007199254740992),
      ]),
      (value) {
        final number = value as int;

        expect(parseCount(formatNumber(number.toDouble())), number);
      },
    );
  });

  property('小数第 1 位までの小数は表示と読み取りを行き来できる', () {
    forAll(
      oneOf([
        integer(min: 0, max: 4294967295).map((tenths) => tenths / 10),
        constant(1e10),
        constant(1e12),
        constant(1e14),
        constant(1e20),
        constant(9.999999999999999e20),
      ]),
      (value) {
        final number = value as double;

        expect(parseDecimal(formatNumber(number)), number);
      },
    );
  });
}
