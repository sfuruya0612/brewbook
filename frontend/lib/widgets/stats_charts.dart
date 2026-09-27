/// 統計のグラフの描画 (FR-18)。
///
/// グラフは fl_chart で描く (ADR-0007)。集計結果に無い区間は描かず、条件が null の抽出は
/// その散布図に描かない。色は chart-count (件数と金額) と chart-grams (グラム、散布図の点、
/// 折れ線) の 2 つだけ (docs/design/components/Charts)。
library;

import 'dart:math' as math;

import 'package:fl_chart/fl_chart.dart';
import 'package:flutter/material.dart';

import '../api/models.dart';
import '../l10n/app_localizations.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';

/// グラフの高さ。
const double _chartHeight = 200;

/// 散布図の高さ (2 列に並べるため棒グラフより低くする)。
const double _scatterHeight = 150;

/// 棒グラフの棒の幅。
const double _barWidth = 12;

/// 目盛の文字 (mono の 11 px、ink-muted)。
TextStyle _axisStyle(BuildContext context) =>
    AppTextStyle.chartAxis(color: BrewbookTheme.of(context).palette.inkMuted);

/// 統計の合計のタイル (value-large と単位。docs/design/components/Stats)。
class StatTile extends StatelessWidget {
  const StatTile({super.key, required this.label, required this.value, this.unit});

  /// 項目名 (ARB から取る)。
  final String label;

  /// 値。数値は整形済みの文字列にする。
  final String value;

  /// 単位 (杯、g など)。
  final String? unit;

  @override
  Widget build(BuildContext context) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final String? unitText = unit;
    final String? unitSuffix = unitText == null ? null : ' $unitText';
    return Container(
      padding: const EdgeInsets.symmetric(
        horizontal: AppSpacing.x4,
        vertical: AppSpacing.x3,
      ),
      decoration: BoxDecoration(
        color: brewbook.palette.paperRaised,
        border: Border.all(color: brewbook.line),
        borderRadius: AppRadius.mdAll,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(label, style: AppTextStyle.caption(color: brewbook.palette.inkMuted)),
          const SizedBox(height: AppSpacing.x1),
          Text.rich(
            TextSpan(
              children: <InlineSpan>[
                TextSpan(
                  text: value,
                  style: AppTextStyle.valueLarge(color: brewbook.palette.ink),
                ),
                if (unitSuffix != null)
                  TextSpan(
                    text: unitSuffix,
                    style: AppTextStyle.caption(color: brewbook.palette.inkMuted),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// 合計のタイルを 2 列に並べる。
class StatTiles extends StatelessWidget {
  const StatTiles({super.key, required this.tiles});

  /// 並べるタイル。
  final List<Widget> tiles;

  @override
  Widget build(BuildContext context) {
    return IntrinsicHeight(
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          for (int index = 0; index < tiles.length; index++) ...<Widget>[
            if (index > 0) const SizedBox(width: AppSpacing.x3),
            Expanded(child: tiles[index]),
          ],
        ],
      ),
    );
  }
}

/// 統計のグラフの区画。見出しと単位、グラフを縦に並べ、下に line の罫線を引く。
class _ChartSection extends StatelessWidget {
  const _ChartSection({required this.title, this.unit, required this.child, this.showDivider = true});

  /// 区画の見出し (ARB から取る)。
  final String title;

  /// 右端の単位 (「杯 / 日」など)。無いときは出さない。
  final String? unit;

  /// グラフ。記録が無いときはその旨の表示を渡す。
  final Widget child;

  /// 下に罫線を引くか。
  final bool showDivider;

  @override
  Widget build(BuildContext context) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final String? unitText = unit;
    return Container(
      padding: const EdgeInsets.only(bottom: AppSpacing.x4),
      decoration: showDivider
          ? BoxDecoration(border: Border(bottom: BorderSide(color: brewbook.line)))
          : null,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Row(
            crossAxisAlignment: CrossAxisAlignment.baseline,
            textBaseline: TextBaseline.alphabetic,
            children: <Widget>[
              Expanded(
                child: Text(title, style: AppTextStyle.heading(color: brewbook.palette.ink)),
              ),
              if (unitText != null)
                Text(unitText, style: AppTextStyle.chartAxis(color: brewbook.palette.inkMuted)),
            ],
          ),
          const SizedBox(height: AppSpacing.x2),
          SizedBox(height: _chartHeight, child: child),
        ],
      ),
    );
  }
}

/// 記録が無い区画の表示 (empty の形)。
Widget _noData(BuildContext context) {
  return Center(
    child: Text(
      AppLocalizations.of(context).noRecords,
      style: AppTextStyle.caption(color: BrewbookTheme.of(context).palette.inkMuted),
    ),
  );
}

/// 区間のキーから粒度 (日別か月別か) を返す。
bool _isDailyKey(String key) => key.length == 10;

/// 日別か月別かの単位の期間の表示 (日 / 月)。
String _periodUnit(AppLocalizations l10n, List<String> keys) {
  if (keys.isEmpty || _isDailyKey(keys.first)) {
    return l10n.statsPeriodDay;
  }
  return l10n.statsPeriodMonth;
}

/// 抽出回数と豆の消費量の棒グラフ (FR-18)。
class BrewStatsCharts extends StatelessWidget {
  const BrewStatsCharts({super.key, required this.periods});

  /// 区間ごとの集計の結果。記録の無い区間は含まれない。
  final List<BrewPeriod> periods;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final List<String> keys = periods.map((period) => period.period).toList();
    final String periodUnit = _periodUnit(l10n, keys);
    final int brewCount = periods.fold(0, (sum, period) => sum + period.brewCount);
    final double dose = periods.fold(0.0, (sum, period) => sum + period.doseGrams);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        StatTiles(
          tiles: <Widget>[
            StatTile(
              label: l10n.statsBrewCountTitle,
              value: brewCount.toString(),
              unit: l10n.statsUnitCups,
            ),
            StatTile(
              label: l10n.statsBrewDoseTitle,
              value: dose.toStringAsFixed(1),
              unit: l10n.gramUnit,
            ),
          ],
        ),
        const SizedBox(height: AppSpacing.x6),
        _ChartSection(
          title: l10n.statsBrewCountTitle,
          unit: l10n.statsChartUnit(l10n.statsUnitCups, periodUnit),
          child: periods.isEmpty
              ? _noData(context)
              : BarChart(
                  _barChartData(
                    context: context,
                    periods: keys,
                    values: periods.map((period) => period.brewCount.toDouble()).toList(),
                    color: brewbook.chartCount,
                  ),
                  key: const Key('stats-brew-count-chart'),
                ),
        ),
        const SizedBox(height: AppSpacing.x6),
        _ChartSection(
          title: l10n.statsBrewDoseTitle,
          unit: l10n.statsChartUnit(l10n.gramUnit, periodUnit),
          showDivider: false,
          child: periods.isEmpty
              ? _noData(context)
              : BarChart(
                  _barChartData(
                    context: context,
                    periods: keys,
                    values: periods.map((period) => period.doseGrams).toList(),
                    color: brewbook.chartGrams,
                  ),
                  key: const Key('stats-brew-dose-chart'),
                ),
        ),
      ],
    );
  }
}

/// 購入金額と重量の棒グラフ (通貨コードごとに分ける。FR-18)。
///
/// 為替換算はせず、通貨コードごとに別のグラフにする (PRD のやらないこと)。
/// 通貨はグラフを分けて題に付ける (色で分けない)。
class PurchaseStatsCharts extends StatelessWidget {
  const PurchaseStatsCharts({super.key, required this.purchases});

  /// 区間と通貨コードの組ごとの集計の結果。記録の無い区間は含まれない。
  final List<PurchasePeriod> purchases;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    if (purchases.isEmpty) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          _ChartSection(
            title: l10n.statsPurchaseAmountLabel,
            child: _noData(context),
          ),
          const SizedBox(height: AppSpacing.x6),
          _ChartSection(
            title: l10n.statsPurchaseWeightLabel,
            showDivider: false,
            child: _noData(context),
          ),
        ],
      );
    }
    // 通貨コードごとに分ける。同じ通貨の行は区間ごとに 1 つだけ返る。
    final groups = <String?, List<PurchasePeriod>>{};
    for (final row in purchases) {
      groups.putIfAbsent(row.priceCurrency, () => <PurchasePeriod>[]).add(row);
    }
    final List<Widget> sections = <Widget>[];
    for (final group in groups.entries) {
      final List<String> keys = group.value.map((row) => row.period).toList();
      final String periodUnit = _periodUnit(l10n, keys);
      final String currency = _currencyLabel(l10n, group.key);
      sections.add(
        _ChartSection(
          title: l10n.statsPurchaseAmountTitle(currency),
          unit: l10n.statsChartUnit(currency, periodUnit),
          child: BarChart(
            _barChartData(
              context: context,
              periods: keys,
              values: group.value.map((row) => row.priceAmount.toDouble()).toList(),
              color: brewbook.chartCount,
            ),
            key: Key('stats-purchase-amount-chart-${_currencyKey(group.key)}'),
          ),
        ),
      );
      sections.add(const SizedBox(height: AppSpacing.x6));
      sections.add(
        _ChartSection(
          title: l10n.statsPurchaseWeightTitle(currency),
          unit: l10n.statsChartUnit(l10n.gramUnit, periodUnit),
          child: BarChart(
            _barChartData(
              context: context,
              periods: keys,
              values: group.value.map((row) => row.weightGrams.toDouble()).toList(),
              color: brewbook.chartGrams,
            ),
            key: Key('stats-purchase-weight-chart-${_currencyKey(group.key)}'),
          ),
        ),
      );
      sections.add(const SizedBox(height: AppSpacing.x6));
    }
    // 最後の区画の下には罫線を引かない。
    if (sections.isNotEmpty) {
      sections.removeLast();
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: sections,
    );
  }

  /// 通貨コードの表示。価格が無い購入は通貨コードが null になる。
  String _currencyLabel(AppLocalizations l10n, String? currency) {
    return currency ?? l10n.statsCurrencyNone;
  }

  /// グラフのキーに使う通貨コード。null は `none` にする。
  String _currencyKey(String? currency) => currency ?? 'none';
}

/// 抽出条件と評価の関係の散布図 (豆の量、湯量、湯の温度、時間。FR-18)。
class BrewRatingScatterCharts extends StatelessWidget {
  const BrewRatingScatterCharts({super.key, required this.ratings});

  /// 期間内の評価を持つ抽出。条件が null の項目はその散布図に描かない。
  final List<BrewRating> ratings;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Text(l10n.statsScatterSection, style: AppTextStyle.heading(color: brewbook.palette.ink)),
        const SizedBox(height: AppSpacing.x3),
        _scatterRow(
          context,
          key1: const Key('stats-rating-dose-chart'),
          title1: l10n.statsRatingDoseTitle,
          spots1: <FlSpot>[
            for (final rating in ratings)
              if (rating.doseGrams != null) FlSpot(rating.doseGrams!, rating.rating.toDouble()),
          ],
          key2: const Key('stats-rating-water-chart'),
          title2: l10n.statsRatingWaterTitle,
          spots2: <FlSpot>[
            for (final rating in ratings)
              if (rating.waterGrams != null)
                FlSpot(rating.waterGrams!, rating.rating.toDouble()),
          ],
          color: brewbook.chartGrams,
        ),
        const SizedBox(height: AppSpacing.x4),
        _scatterRow(
          context,
          key1: const Key('stats-rating-temp-chart'),
          title1: l10n.statsRatingTempTitle,
          spots1: <FlSpot>[
            for (final rating in ratings)
              if (rating.waterTempC != null)
                FlSpot(rating.waterTempC!, rating.rating.toDouble()),
          ],
          key2: const Key('stats-rating-time-chart'),
          title2: l10n.statsRatingTimeTitle,
          spots2: <FlSpot>[
            for (final rating in ratings)
              if (rating.brewTimeSeconds != null)
                FlSpot(rating.brewTimeSeconds!.toDouble(), rating.rating.toDouble()),
          ],
          color: brewbook.chartGrams,
        ),
      ],
    );
  }

  /// 散布図を 2 列に並べる。
  Widget _scatterRow(
    BuildContext context, {
    required Key key1,
    required String title1,
    required List<FlSpot> spots1,
    required Key key2,
    required String title2,
    required List<FlSpot> spots2,
    required Color color,
  }) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Expanded(child: _scatter(context, key: key1, title: title1, spots: spots1, color: color)),
        const SizedBox(width: AppSpacing.x3),
        Expanded(child: _scatter(context, key: key2, title: title2, spots: spots2, color: color)),
      ],
    );
  }

  /// 条件と評価の散布図。条件が null の抽出は渡さない。
  Widget _scatter(
    BuildContext context, {
    required Key key,
    required String title,
    required List<FlSpot> spots,
    required Color color,
  }) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Text(title, style: AppTextStyle.caption(color: brewbook.palette.inkMuted)),
        const SizedBox(height: AppSpacing.x1),
        SizedBox(
          height: _scatterHeight,
          child: spots.isEmpty
              ? _noData(context)
              : ScatterChart(_scatterChartData(context, spots, color), key: key),
        ),
      ],
    );
  }
}

/// 購入ごとの評価の推移の折れ線グラフ (FR-18)。
class RatingHistoryChart extends StatelessWidget {
  const RatingHistoryChart({super.key, required this.entries});

  /// 購入に紐づく評価を持つ抽出。抽出日時の昇順で並ぶ。
  final List<RatingHistoryEntry> entries;

  @override
  Widget build(BuildContext context) {
    if (entries.isEmpty) {
      return SizedBox(height: _chartHeight, child: _noData(context));
    }
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    return SizedBox(
      height: _chartHeight,
      child: LineChart(
        _lineChartData(context, entries, brewbook.chartGrams, brewbook.palette.paperRaised),
        key: const Key('purchase-rating-history-chart'),
      ),
    );
  }
}

/// 区間ごとの棒グラフを組み立てる。
///
/// 受け取った区間だけを描くため、記録の無い区間は現れない。
BarChartData _barChartData({
  required BuildContext context,
  required List<String> periods,
  required List<double> values,
  required Color color,
}) {
  final BrewbookTheme brewbook = BrewbookTheme.of(context);
  final double maxValue = values.isEmpty ? 0 : values.reduce(math.max);
  final double maxY = maxValue <= 0 ? 1 : maxValue * 1.1;
  return BarChartData(
    minY: 0,
    maxY: maxY,
    barGroups: <BarChartGroupData>[
      for (int index = 0; index < periods.length; index++)
        BarChartGroupData(
          x: index,
          barRods: <BarChartRodData>[
            BarChartRodData(
              toY: values[index],
              width: _barWidth,
              color: color,
              borderRadius: const BorderRadius.vertical(top: Radius.circular(1)),
            ),
          ],
        ),
    ],
    titlesData: _titles(
      context,
      keys: periods,
      maxY: maxY,
      // 日別は先頭、中央、末尾の 3 つだけラベルを出す。
      labelIndices: _labelIndices(periods.length),
    ),
    gridData: FlGridData(
      show: true,
      drawVerticalLine: false,
      horizontalInterval: maxY / 2,
      // 目盛の横罫は最大と半分の 2 本にする (0 は軸)。
      checkToShowHorizontalLine: (value) => value > 0,
      getDrawingHorizontalLine: (value) => FlLine(color: brewbook.line, strokeWidth: 1),
    ),
    borderData: FlBorderData(
      show: true,
      border: Border(bottom: BorderSide(color: brewbook.lineStrong, width: 1)),
    ),
  );
}

/// 条件と評価の散布図を組み立てる。
ScatterChartData _scatterChartData(BuildContext context, List<FlSpot> spots, Color color) {
  var minX = spots.first.x;
  var maxX = spots.first.x;
  for (final spot in spots) {
    minX = math.min(minX, spot.x);
    maxX = math.max(maxX, spot.x);
  }
  if (minX == maxX) {
    // 条件が 1 つだけでも軸が潰れないようにする。
    minX -= 1;
    maxX += 1;
  }
  return ScatterChartData(
    scatterSpots: <ScatterSpot>[
      for (final spot in spots)
        ScatterSpot(
          spot.x,
          spot.y,
          dotPainter: FlDotCirclePainter(color: color.withValues(alpha: 0.85), radius: 3.5),
        ),
    ],
    minX: minX,
    maxX: maxX,
    // 評価は 1 から 5 (FR-18)。
    minY: 1,
    maxY: 5,
    titlesData: _titles(context, keys: const <String>[], labelIndices: const <int>{}),
    gridData: const FlGridData(show: true, drawVerticalLine: false),
    borderData: FlBorderData(show: false),
  );
}

/// 評価の推移の折れ線グラフを組み立てる。
LineChartData _lineChartData(
  BuildContext context,
  List<RatingHistoryEntry> entries,
  Color color,
  Color dotFill,
) {
  return LineChartData(
    lineBarsData: <LineChartBarData>[
      LineChartBarData(
        spots: <FlSpot>[
          for (var index = 0; index < entries.length; index++)
            FlSpot(index.toDouble(), entries[index].rating.toDouble()),
        ],
        isCurved: false,
        color: color,
        barWidth: 2,
        dotData: FlDotData(
          show: true,
          getDotPainter: (spot, percent, bar, index) => FlDotCirclePainter(
            radius: 4,
            color: dotFill,
            strokeWidth: 2,
            strokeColor: color,
          ),
        ),
      ),
    ],
    minX: 0,
    maxX: entries.length <= 1 ? 1 : (entries.length - 1).toDouble(),
    // 評価は 1 から 5 (FR-18)。
    minY: 1,
    maxY: 5,
    titlesData: _titles(
      context,
      keys: entries.map((entry) => entry.brewedAt).toList(),
      // 横軸は最初と最後の日付だけ。
      labelIndices: entries.length <= 1 ? <int>{0} : <int>{0, entries.length - 1},
    ),
    gridData: const FlGridData(show: false),
    borderData: FlBorderData(show: false),
  );
}

/// 下の軸にラベルを出す位置。中央は先頭と末尾と重なるときは省く。
Set<int> _labelIndices(int length) {
  if (length <= 3) {
    return <int>{for (var index = 0; index < length; index++) index};
  }
  return <int>{0, length ~/ 2, length - 1};
}

/// 軸の表示。上と右は出さず、左は最大と半分、下は区間のキーを出す。
FlTitlesData _titles(BuildContext context, {required List<String> keys, required Set<int> labelIndices, double? maxY}) {
  final TextStyle style = _axisStyle(context);
  return FlTitlesData(
    topTitles: const AxisTitles(sideTitles: SideTitles(showTitles: false)),
    rightTitles: const AxisTitles(sideTitles: SideTitles(showTitles: false)),
    leftTitles: AxisTitles(
      sideTitles: SideTitles(
        showTitles: true,
        reservedSize: 44,
        interval: maxY == null ? null : maxY / 2,
        getTitlesWidget: (value, meta) => SideTitleWidget(
          meta: meta,
          child: Text(_formatAxis(value), style: style),
        ),
      ),
    ),
    bottomTitles: AxisTitles(
      sideTitles: SideTitles(
        showTitles: keys.isNotEmpty,
        reservedSize: 30,
        getTitlesWidget: (value, meta) {
          final index = value.toInt();
          // 目盛りは整数の位置だけに出す (間引かれた位置の値は表示しない)。
          if (value != index.toDouble() || index < 0 || index >= keys.length) {
            return const SizedBox.shrink();
          }
          // 日別は先頭、中央、末尾の 3 つだけラベルを出す。
          if (!labelIndices.contains(index)) {
            return const SizedBox.shrink();
          }
          return SideTitleWidget(
            meta: meta,
            child: Text(_keyLabel(keys[index]), style: style),
          );
        },
      ),
    ),
  );
}

/// 軸の目盛の文字。整数のときはそのまま、そうでないときは小数第 1 位まで出す。
String _formatAxis(double value) {
  if (value == value.roundToDouble()) {
    return value.round().toString();
  }
  return value.toStringAsFixed(1);
}

/// 区間のキーまたは日時を軸のラベルにする。日別は月日だけを出す。
String _keyLabel(String key) {
  final day = DateTime.tryParse(key);
  if (day != null) {
    final local = day.toLocal();
    return '${local.month.toString().padLeft(2, '0')}-${local.day.toString().padLeft(2, '0')}';
  }
  // `YYYY-MM` の月別のキーはそのまま出す。
  if (key.length == 10) {
    return key.substring(5);
  }
  return key;
}
