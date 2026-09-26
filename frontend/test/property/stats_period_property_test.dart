// 統計の期間の組み立ての PBT (issue 0027、ADR-0013)。
//
// 端末のタイムゾーンでの現在の日時 (年 2000 から 2100、日は 1 から 28) を生成し、
// statsPeriodFor と statsDayCount の性質を検証する。日付の加算はタイムゾーンと
// 夏時間の影響を受けないように、日付の部品を組み立て直して行う。反例が出たときは、
// 失敗の出力にある seed の値を forAll の seed に指定して再現する。
import 'package:brew_book/api/stats_api.dart';
import 'package:brew_book/records/stats_period.dart';
import 'package:brew_book/records/values.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:kiri_check/kiri_check.dart';

/// 実在する日付の部品 (年 2000 から 2100、月 1 から 12、日 1 から 28)。
Arbitrary<(int, int, int)> dayParts() => combine3(
      integer(min: 2000, max: 2100),
      integer(min: 1, max: 12),
      integer(min: 1, max: 28),
    );

/// 実在する日付。
Arbitrary<DateTime> day() => dayParts().map(
      (parts) {
        final (year, month, dayOfMonth) = parts;
        return DateTime(year, month, dayOfMonth);
      },
    );

/// 日付の部品を組み立て直して日数を足す (絶対時間の加算を避ける)。
DateTime addDays(DateTime day, int days) =>
    DateTime(day.year, day.month, day.day + days);

void main() {
  property('当月は月初から当日までの日別になる', () {
    forAll(day(), (now) {
      final period = statsPeriodFor(StatsPeriodPreset.currentMonth, now);

      expect(period.start, formatDay(DateTime(now.year, now.month, 1)));
      expect(period.end, formatDay(now));
      expect(period.granularity, StatsGranularity.day);
    });
  });

  property('3 か月、6 か月、12 か月は当月を含む直近の月数になる', () {
    forAll(day(), (now) {
      for (final (preset, months) in [
        (StatsPeriodPreset.threeMonths, 3),
        (StatsPeriodPreset.sixMonths, 6),
        (StatsPeriodPreset.twelveMonths, 12),
      ]) {
        final period = statsPeriodFor(preset, now);

        expect(
          period.start,
          formatDay(DateTime(now.year, now.month - (months - 1), 1)),
        );
        expect(period.end, formatDay(now));
        expect(period.granularity, StatsGranularity.month);
      }
    });
  });

  property('任意の期間は 62 日以下なら日別、63 日以上なら月別になる', () {
    forAll(
      combine2(day(), integer(min: 0, max: 200)),
      (input) {
        final (start, days) = input;
        final end = addDays(start, days);
        final period = statsPeriodFor(
          StatsPeriodPreset.custom,
          start,
          customStart: start,
          customEnd: end,
        );

        expect(period.start, formatDay(start));
        expect(period.end, formatDay(end));
        expect(
          period.granularity,
          statsDayCount(start, end) <= statsDayGranularityMaxDays
              ? StatsGranularity.day
              : StatsGranularity.month,
        );
      },
    );
  });

  property('62 日は日別、63 日は月別になる (境界の固定値)', () {
    final start = DateTime(2026, 1, 1);
    final day62 = addDays(start, 61);
    final day63 = addDays(start, 62);

    expect(statsDayCount(start, day62), 62);
    expect(
      statsPeriodFor(
        StatsPeriodPreset.custom,
        start,
        customStart: start,
        customEnd: day62,
      ).granularity,
      StatsGranularity.day,
    );
    expect(statsDayCount(start, day63), 63);
    expect(
      statsPeriodFor(
        StatsPeriodPreset.custom,
        start,
        customStart: start,
        customEnd: day63,
      ).granularity,
      StatsGranularity.month,
    );
  });

  property('終了日が開始日より前の任意の期間は例外になる', () {
    forAll(
      combine2(day(), integer(min: 1, max: 200)),
      (input) {
        final (start, days) = input;
        final end = addDays(start, -days);

        expect(
          () => statsPeriodFor(
            StatsPeriodPreset.custom,
            start,
            customStart: start,
            customEnd: end,
          ),
          throwsArgumentError,
        );
      },
    );
  });

  property('statsDayCount は両端を含む日数になる', () {
    forAll(
      combine2(day(), integer(min: 0, max: 365)),
      (input) {
        final (start, days) = input;
        final end = addDays(start, days);

        expect(statsDayCount(start, end), days + 1);
      },
    );
  });

  property('statsDayCount は終了日が開始日より前なら正の値にならない', () {
    forAll(
      combine2(day(), integer(min: 1, max: 365)),
      (input) {
        final (parts, days) = input;
        final start = parts;
        final end = addDays(start, -days);

        expect(statsDayCount(start, end) <= 0, isTrue);
      },
    );
  });
}
