/// 統計の期間と粒度の組み立て (FR-18)。
///
/// 画面の期間の切り替え (当月、3 か月、6 か月、12 か月、全期間、任意の開始日と終了日) を、
/// API に渡す開始日、終了日、粒度に変換する。日付は端末のタイムゾーンでのローカル日付とする。
library;

import '../api/stats_api.dart';
import 'values.dart';

/// 任意の期間を日別で表示する上限の日数 (FR-18)。
const int statsDayGranularityMaxDays = 62;

/// 統計画面の期間の切り替えの種別 (FR-18)。
enum StatsPeriodPreset {
  /// 当月 (端末のタイムゾーンでの当月 1 日から当日まで。日別)。
  currentMonth,

  /// 当月を含む直近の 3 か月 (月別)。
  threeMonths,

  /// 当月を含む直近の 6 か月 (月別)。
  sixMonths,

  /// 当月を含む直近の 12 か月 (月別)。
  twelveMonths,

  /// 全期間 (開始日と終了日を省略する。月別)。
  allTime,

  /// 任意の開始日と終了日 (62 日以下なら日別、それ以外は月別)。
  custom,
}

/// API に渡す期間と粒度 (FR-18)。
class StatsPeriod {
  const StatsPeriod({this.start, this.end, required this.granularity});

  /// 開始日 (`YYYY-MM-DD`)。全期間では null。
  final String? start;

  /// 終了日 (`YYYY-MM-DD`)。全期間では null。
  final String? end;

  /// 集計の粒度。
  final StatsGranularity granularity;
}

/// 切り替えの種別から期間と粒度を組み立てる (FR-18)。
///
/// [now] は端末のタイムゾーンでの現在の日時。[customStart] と [customEnd] は
/// [StatsPeriodPreset.custom] のときだけ使う。
StatsPeriod statsPeriodFor(
  StatsPeriodPreset preset,
  DateTime now, {
  DateTime? customStart,
  DateTime? customEnd,
}) {
  switch (preset) {
    case StatsPeriodPreset.currentMonth:
      return StatsPeriod(
        start: formatDay(DateTime(now.year, now.month, 1)),
        end: formatDay(now),
        granularity: StatsGranularity.day,
      );
    case StatsPeriodPreset.threeMonths:
      return _recentMonths(now, 3);
    case StatsPeriodPreset.sixMonths:
      return _recentMonths(now, 6);
    case StatsPeriodPreset.twelveMonths:
      return _recentMonths(now, 12);
    case StatsPeriodPreset.allTime:
      return const StatsPeriod(granularity: StatsGranularity.month);
    case StatsPeriodPreset.custom:
      if (customStart == null || customEnd == null) {
        throw ArgumentError('a custom period needs a start and an end');
      }
      if (formatDay(customEnd).compareTo(formatDay(customStart)) < 0) {
        throw ArgumentError('a custom period needs the end on or after the start');
      }
      return StatsPeriod(
        start: formatDay(customStart),
        end: formatDay(customEnd),
        granularity: statsDayCount(customStart, customEnd) <= statsDayGranularityMaxDays
            ? StatsGranularity.day
            : StatsGranularity.month,
      );
  }
}

/// 開始日から終了日までの日数 (両端を含む)。
///
/// 日付だけの値で数えるため、夏時間の切り替えの影響を受けないよう UTC の日付に直してから
/// 差を取る。
int statsDayCount(DateTime start, DateTime end) {
  final from = DateTime.utc(start.year, start.month, start.day);
  final to = DateTime.utc(end.year, end.month, end.day);
  return to.difference(from).inDays + 1;
}

/// 当月を含む直近 [months] か月の月別の期間。
StatsPeriod _recentMonths(DateTime now, int months) {
  final start = DateTime(now.year, now.month - (months - 1), 1);
  return StatsPeriod(
    start: formatDay(start),
    end: formatDay(now),
    granularity: StatsGranularity.month,
  );
}
