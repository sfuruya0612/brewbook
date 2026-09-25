/// 統計の期間と粒度の組み立ての単体テスト (FR-18)。
///
/// 期間の切り替えの種別が、API に渡す開始日、終了日、粒度になることを確認する。
library;

import 'package:coffee_log/api/stats_api.dart';
import 'package:coffee_log/records/stats_period.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  // 端末のタイムゾーンでの現在の日時。月と年の境界の確認に使う。
  final now = DateTime(2026, 9, 25, 12, 30);

  test('当月は 1 日から当日までを日別にする', () {
    final period = statsPeriodFor(StatsPeriodPreset.currentMonth, now);

    expect(period.start, '2026-09-01');
    expect(period.end, '2026-09-25');
    expect(period.granularity, StatsGranularity.day);
  });

  test('3 か月、6 か月、12 か月は当月を含む直近の月数を月別にする', () {
    final three = statsPeriodFor(StatsPeriodPreset.threeMonths, now);
    expect(three.start, '2026-07-01');
    expect(three.end, '2026-09-25');
    expect(three.granularity, StatsGranularity.month);

    final six = statsPeriodFor(StatsPeriodPreset.sixMonths, now);
    expect(six.start, '2026-04-01');
    expect(six.end, '2026-09-25');
    expect(six.granularity, StatsGranularity.month);

    final twelve = statsPeriodFor(StatsPeriodPreset.twelveMonths, now);
    expect(twelve.start, '2025-10-01');
    expect(twelve.end, '2026-09-25');
    expect(twelve.granularity, StatsGranularity.month);
  });

  test('年をまたぐ月数も当月を含む直近の月数にする', () {
    final period = statsPeriodFor(StatsPeriodPreset.threeMonths, DateTime(2026, 1, 15));

    expect(period.start, '2025-11-01');
    expect(period.end, '2026-01-15');
  });

  test('全期間は開始日と終了日を省略して月別にする', () {
    final period = statsPeriodFor(StatsPeriodPreset.allTime, now);

    expect(period.start, isNull);
    expect(period.end, isNull);
    expect(period.granularity, StatsGranularity.month);
  });

  test('任意の期間は 62 日以下なら日別、それ以外は月別にする', () {
    // 2026-07-01 から 2026-08-31 までは 62 日 (両端を含む)。
    final short = statsPeriodFor(
      StatsPeriodPreset.custom,
      now,
      customStart: DateTime(2026, 7, 1),
      customEnd: DateTime(2026, 8, 31),
    );
    expect(short.start, '2026-07-01');
    expect(short.end, '2026-08-31');
    expect(short.granularity, StatsGranularity.day);
    expect(statsDayCount(DateTime(2026, 7, 1), DateTime(2026, 8, 31)), 62);

    // 1 日増えると月別にする。
    final long = statsPeriodFor(
      StatsPeriodPreset.custom,
      now,
      customStart: DateTime(2026, 7, 1),
      customEnd: DateTime(2026, 9, 1),
    );
    expect(long.granularity, StatsGranularity.month);
    expect(statsDayCount(DateTime(2026, 7, 1), DateTime(2026, 9, 1)), 63);
  });

  test('任意の期間に開始日と終了日が無ければ組み立てない', () {
    expect(
      () => statsPeriodFor(StatsPeriodPreset.custom, now),
      throwsArgumentError,
    );
  });

  test('任意の期間の終了日が開始日より前なら組み立てない', () {
    expect(
      () => statsPeriodFor(
        StatsPeriodPreset.custom,
        now,
        customStart: DateTime(2026, 9, 2),
        customEnd: DateTime(2026, 9, 1),
      ),
      throwsArgumentError,
    );
  });
}
