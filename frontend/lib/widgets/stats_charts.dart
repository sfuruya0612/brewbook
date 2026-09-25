/// 統計のグラフの描画 (FR-18)。
///
/// グラフは fl_chart で描く (ADR-0007)。集計結果に無い区間は描かず、条件が null の抽出は
/// その散布図に描かない。
library;

import 'dart:math' as math;

import 'package:fl_chart/fl_chart.dart';
import 'package:flutter/material.dart';

import '../api/models.dart';
import '../l10n/app_localizations.dart';

/// グラフの高さ。
const double _chartHeight = 200;

/// 棒グラフの棒の幅。
const double _barWidth = 12;

/// 統計のグラフの区画。見出しとグラフを縦に並べる。
class _ChartSection extends StatelessWidget {
  const _ChartSection({required this.title, required this.child});

  /// 区画の見出し。
  final String title;

  /// グラフ。記録が無いときはその旨の表示を渡す。
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(title, style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          SizedBox(height: _chartHeight, child: child),
        ],
      ),
    );
  }
}

/// 記録が無い区画の表示。
Widget _noData(BuildContext context) {
  return Center(child: Text(AppLocalizations.of(context).noRecords));
}

/// 抽出回数と豆の消費量の棒グラフ (FR-18)。
class BrewStatsCharts extends StatelessWidget {
  const BrewStatsCharts({super.key, required this.periods});

  /// 区間ごとの集計の結果。記録の無い区間は含まれない。
  final List<BrewPeriod> periods;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final color = Theme.of(context).colorScheme.primary;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        _ChartSection(
          title: l10n.statsBrewCountTitle,
          child: periods.isEmpty
              ? _noData(context)
              : BarChart(
                  _barChartData(
                    periods: periods.map((period) => period.period).toList(),
                    values: periods.map((period) => period.brewCount.toDouble()).toList(),
                    color: color,
                  ),
                  key: const Key('stats-brew-count-chart'),
                ),
        ),
        _ChartSection(
          title: l10n.statsBrewDoseTitle,
          child: periods.isEmpty
              ? _noData(context)
              : BarChart(
                  _barChartData(
                    periods: periods.map((period) => period.period).toList(),
                    values: periods.map((period) => period.doseGrams).toList(),
                    color: color,
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
class PurchaseStatsCharts extends StatelessWidget {
  const PurchaseStatsCharts({super.key, required this.purchases});

  /// 区間と通貨コードの組ごとの集計の結果。記録の無い区間は含まれない。
  final List<PurchasePeriod> purchases;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    if (purchases.isEmpty) {
      return Padding(
        padding: const EdgeInsets.all(16),
        child: Text(l10n.noRecords),
      );
    }
    final color = Theme.of(context).colorScheme.primary;
    // 通貨コードごとに分ける。同じ通貨の行は区間ごとに 1 つだけ返る。
    final groups = <String?, List<PurchasePeriod>>{};
    for (final row in purchases) {
      groups.putIfAbsent(row.priceCurrency, () => <PurchasePeriod>[]).add(row);
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        for (final group in groups.entries) ...<Widget>[
          _ChartSection(
            title: l10n.statsPurchaseAmountTitle(_currencyLabel(l10n, group.key)),
            child: BarChart(
              _barChartData(
                periods: group.value.map((row) => row.period).toList(),
                values: group.value.map((row) => row.priceAmount.toDouble()).toList(),
                color: color,
              ),
              key: Key('stats-purchase-amount-chart-${_currencyKey(group.key)}'),
            ),
          ),
          _ChartSection(
            title: l10n.statsPurchaseWeightTitle(_currencyLabel(l10n, group.key)),
            child: BarChart(
              _barChartData(
                periods: group.value.map((row) => row.period).toList(),
                values: group.value.map((row) => row.weightGrams.toDouble()).toList(),
                color: color,
              ),
              key: Key('stats-purchase-weight-chart-${_currencyKey(group.key)}'),
            ),
          ),
        ],
      ],
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
    final color = Theme.of(context).colorScheme.secondary;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        _ChartSection(
          title: l10n.statsRatingDoseTitle,
          child: _scatter(
            key: const Key('stats-rating-dose-chart'),
            spots: <FlSpot>[
              for (final rating in ratings)
                if (rating.doseGrams != null) FlSpot(rating.doseGrams!, rating.rating.toDouble()),
            ],
            color: color,
          ),
        ),
        _ChartSection(
          title: l10n.statsRatingWaterTitle,
          child: _scatter(
            key: const Key('stats-rating-water-chart'),
            spots: <FlSpot>[
              for (final rating in ratings)
                if (rating.waterGrams != null)
                  FlSpot(rating.waterGrams!, rating.rating.toDouble()),
            ],
            color: color,
          ),
        ),
        _ChartSection(
          title: l10n.statsRatingTempTitle,
          child: _scatter(
            key: const Key('stats-rating-temp-chart'),
            spots: <FlSpot>[
              for (final rating in ratings)
                if (rating.waterTempC != null)
                  FlSpot(rating.waterTempC!, rating.rating.toDouble()),
            ],
            color: color,
          ),
        ),
        _ChartSection(
          title: l10n.statsRatingTimeTitle,
          child: _scatter(
            key: const Key('stats-rating-time-chart'),
            spots: <FlSpot>[
              for (final rating in ratings)
                if (rating.brewTimeSeconds != null)
                  FlSpot(rating.brewTimeSeconds!.toDouble(), rating.rating.toDouble()),
            ],
            color: color,
          ),
        ),
      ],
    );
  }

  /// 条件と評価の散布図。条件が null の抽出は渡さない。
  Widget _scatter({required Key key, required List<FlSpot> spots, required Color color}) {
    if (spots.isEmpty) {
      return Builder(builder: _noData);
    }
    return ScatterChart(_scatterChartData(spots, color), key: key);
  }
}

/// 購入ごとの評価の推移の折れ線グラフ (FR-18)。
class RatingHistoryChart extends StatelessWidget {
  const RatingHistoryChart({super.key, required this.entries});

  /// 購入に紐づく評価を持つ抽出。抽出日時の昇順で並ぶ。
  final List<RatingHistoryEntry> entries;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    if (entries.isEmpty) {
      return Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16),
        child: Text(l10n.noRecords),
      );
    }
    return SizedBox(
      height: _chartHeight,
      child: LineChart(
        _lineChartData(entries, Theme.of(context).colorScheme.primary),
        key: const Key('purchase-rating-history-chart'),
      ),
    );
  }
}

/// 区間ごとの棒グラフを組み立てる。
///
/// 受け取った区間だけを描くため、記録の無い区間は現れない。
BarChartData _barChartData({
  required List<String> periods,
  required List<double> values,
  required Color color,
}) {
  final maxValue = values.isEmpty ? 0.0 : values.reduce(math.max);
  return BarChartData(
    minY: 0,
    // 値が 0 以下でも軸が潰れないようにする。
    maxY: maxValue <= 0 ? 1 : maxValue * 1.1,
    barGroups: <BarChartGroupData>[
      for (var index = 0; index < periods.length; index++)
        BarChartGroupData(
          x: index,
          barRods: <BarChartRodData>[
            BarChartRodData(
              toY: values[index],
              width: _barWidth,
              color: color,
              borderRadius: const BorderRadius.vertical(top: Radius.circular(2)),
            ),
          ],
        ),
    ],
    titlesData: _titles(periods),
    gridData: const FlGridData(show: true, drawVerticalLine: false),
    borderData: FlBorderData(show: false),
  );
}

/// 条件と評価の散布図を組み立てる。
ScatterChartData _scatterChartData(List<FlSpot> spots, Color color) {
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
          dotPainter: FlDotCirclePainter(color: color, radius: 4),
        ),
    ],
    minX: minX,
    maxX: maxX,
    // 評価は 1 から 5 (FR-18)。
    minY: 0,
    maxY: 5,
    titlesData: _titles(const <String>[]),
    gridData: const FlGridData(show: true, drawVerticalLine: false),
    borderData: FlBorderData(show: false),
  );
}

/// 評価の推移の折れ線グラフを組み立てる。
LineChartData _lineChartData(List<RatingHistoryEntry> entries, Color color) {
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
        dotData: const FlDotData(show: true),
      ),
    ],
    minX: 0,
    maxX: entries.length <= 1 ? 1 : (entries.length - 1).toDouble(),
    // 評価は 1 から 5 (FR-18)。
    minY: 0,
    maxY: 5,
    titlesData: _titles(entries.map((entry) => entry.brewedAt).toList()),
    gridData: const FlGridData(show: true, drawVerticalLine: false),
    borderData: FlBorderData(show: false),
  );
}

/// 軸の表示。上と右は出さず、下は区間のキーを出す。
FlTitlesData _titles(List<String> keys) {
  return FlTitlesData(
    topTitles: const AxisTitles(sideTitles: SideTitles(showTitles: false)),
    rightTitles: const AxisTitles(sideTitles: SideTitles(showTitles: false)),
    leftTitles: const AxisTitles(
      sideTitles: SideTitles(showTitles: true, reservedSize: 44),
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
          return SideTitleWidget(
            meta: meta,
            child: Text(_keyLabel(keys[index])),
          );
        },
      ),
    ),
  );
}

/// 区間のキーまたは日時を軸のラベルにする。日別は月日だけを出す。
String _keyLabel(String key) {
  final day = DateTime.tryParse(key);
  if (day != null) {
    final local = day.toLocal();
    return '${local.month}/${local.day}';
  }
  // `YYYY-MM` の月別のキーはそのまま出す。
  if (key.length == 10) {
    return key.substring(5);
  }
  return key;
}
