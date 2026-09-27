import 'package:flutter/material.dart';

import '../api/models.dart';
import '../api/stats_api.dart';
import '../l10n/app_localizations.dart';
import '../records/record_services.dart';
import '../records/stats_period.dart';
import '../records/values.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';
import '../widgets/day_time_fields.dart';
import '../widgets/error_banner.dart';
import '../widgets/error_message.dart';
import '../widgets/stats_charts.dart';
import '../widgets/wide_layout.dart';

/// 統計の画面 (FR-18)。
///
/// 初期表示は端末のタイムゾーンでの当月 (1 日から当日まで) を日別で表示し、当月、3 か月、
/// 6 か月、12 か月、全期間、任意の開始日と終了日に切り替えられる。集計は Backend の API が
/// 行い、この画面は集計結果をグラフに描くだけにする (ADR-0007)。
class StatsScreen extends StatefulWidget {
  const StatsScreen({super.key, required this.services});

  /// 記録の画面が使う依存 (ADR-0007)。
  final RecordServices services;

  @override
  State<StatsScreen> createState() => _StatsScreenState();
}

class _StatsScreenState extends State<StatsScreen> {
  StatsPeriodPreset _preset = StatsPeriodPreset.currentMonth;
  late final TextEditingController _startController;
  late final TextEditingController _endController;
  DateTime? _customStart;
  DateTime? _customEnd;
  List<BrewPeriod> _brews = const <BrewPeriod>[];
  List<PurchasePeriod> _purchases = const <PurchasePeriod>[];
  List<BrewRating> _ratings = const <BrewRating>[];
  bool _loading = false;
  String? _errorMessage;

  /// 読み込みの世代。古い読み込みの応答を捨てるために使う。
  int _loadGeneration = 0;
  String? _startError;
  String? _endError;

  @override
  void initState() {
    super.initState();
    // 任意の期間の既定値は当月 (1 日から当日まで)。
    final now = widget.services.clock.now();
    _customStart = DateTime(now.year, now.month, 1);
    _customEnd = now;
    _startController = TextEditingController(text: formatDay(_customStart!));
    _endController = TextEditingController(text: formatDay(_customEnd!));
    _load();
  }

  @override
  void dispose() {
    _startController.dispose();
    _endController.dispose();
    super.dispose();
  }

  /// 選択している期間の集計を読み込む。
  ///
  /// 期間の切り替えが重なると古い読み込みの結果が新しい表示を上書きするため、
  /// 読み込みの世代を持ち、最新の世代の応答だけを反映する。
  Future<void> _load() async {
    final generation = ++_loadGeneration;
    final period = statsPeriodFor(
      _preset,
      widget.services.clock.now(),
      customStart: _customStart,
      customEnd: _customEnd,
    );
    setState(() {
      _loading = true;
      _errorMessage = null;
    });
    try {
      final offset = widget.services.clock.utcOffsetMinutes();
      final brews = await widget.services.stats.brews(
        start: period.start,
        end: period.end,
        granularity: period.granularity,
        utcOffsetMinutes: offset,
      );
      final purchases = await widget.services.stats.purchases(
        start: period.start,
        end: period.end,
        granularity: period.granularity,
      );
      final ratings = await widget.services.stats.brewRatings(
        start: period.start,
        end: period.end,
        utcOffsetMinutes: offset,
      );
      if (!mounted || generation != _loadGeneration) {
        return;
      }
      setState(() {
        _brews = brews;
        _purchases = purchases;
        _ratings = ratings;
      });
    } catch (error) {
      if (!mounted || generation != _loadGeneration) {
        return;
      }
      setState(() => _errorMessage = messageForError(error, AppLocalizations.of(context)));
    } finally {
      if (mounted && generation == _loadGeneration) {
        setState(() => _loading = false);
      }
    }
  }

  /// 期間の切り替えを選ぶ。任意の期間だけは適用を押すまで読み込まない。
  void _selectPreset(StatsPeriodPreset preset) {
    setState(() {
      _preset = preset;
      _startError = null;
      _endError = null;
    });
    if (preset != StatsPeriodPreset.custom) {
      _load();
    }
  }

  /// 任意の期間を検査して読み込む。
  void _applyCustom() {
    final l10n = AppLocalizations.of(context);
    final start = parseDay(_startController.text);
    final end = parseDay(_endController.text);
    setState(() {
      _startError = start == null ? l10n.validationDay : null;
      _endError = end == null
          ? l10n.validationDay
          : start != null && end.isBefore(start)
          ? l10n.validationPeriod
          : null;
    });
    if (start == null || end == null || end.isBefore(start)) {
      return;
    }
    _customStart = start;
    _customEnd = end;
    _load();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(title: Text(l10n.statsTitle)),
      body: LayoutBuilder(
        builder: (context, constraints) {
          final wide = constraints.maxWidth >= wideLayoutBreakpoint;
          final content = Column(
            children: <Widget>[
              _periodSelector(l10n),
              if (_preset == StatsPeriodPreset.custom) _customFields(l10n),
              Expanded(child: _body(context, l10n, wide)),
            ],
          );
          if (!wide) {
            return content;
          }
          return Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              const BrewbookNavigationRail(selectedIndex: 4),
              Expanded(child: content),
            ],
          );
        },
      ),
    );
  }

  /// 期間の切り替え (FR-18)。
  Widget _periodSelector(AppLocalizations l10n) {
    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      padding: const EdgeInsets.symmetric(
        horizontal: AppSpacing.x4,
        vertical: AppSpacing.x2,
      ),
      child: Row(
        children: <Widget>[
          for (final preset in StatsPeriodPreset.values)
            Padding(
              padding: const EdgeInsets.only(right: AppSpacing.x2),
              child: SizedBox(
                height: 32,
                child: ChoiceChip(
                  key: Key('stats-period-${preset.name}'),
                  label: Text(_presetLabel(l10n, preset)),
                  labelStyle: AppTextStyle.label(
                    color: _preset == preset
                        ? BrewbookTheme.of(context).palette.onRoast
                        : BrewbookTheme.of(context).palette.ink,
                  ),
                  selected: _preset == preset,
                  onSelected: (selected) => _selectPreset(preset),
                  showCheckmark: false,
                ),
              ),
            ),
        ],
      ),
    );
  }

  /// 選択中の範囲と粒度の 1 行 (caption)。
  Widget _rangeCaption(AppLocalizations l10n) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final period = statsPeriodFor(
      _preset,
      widget.services.clock.now(),
      customStart: _customStart,
      customEnd: _customEnd,
    );
    final String granularity = _granularityLabel(l10n, period.granularity);
    final TextStyle dateStyle = AppTextStyle.mono(
      size: 12,
      lineHeight: 16,
      color: brewbook.palette.inkMuted,
    );
    final StatelessWidget caption;
    final String? start = period.start;
    final String? end = period.end;
    if (start == null || end == null) {
      caption = Text(
        l10n.statsRangeAllTime(granularity),
        style: AppTextStyle.caption(color: brewbook.palette.inkMuted),
      );
    } else {
      caption = Text.rich(
        TextSpan(
          style: AppTextStyle.caption(color: brewbook.palette.inkMuted),
          children: <InlineSpan>[
            TextSpan(text: l10n.statsRangePrefix),
            TextSpan(text: start, style: dateStyle),
            TextSpan(text: l10n.statsRangeMiddle),
            TextSpan(text: end, style: dateStyle),
            TextSpan(text: l10n.statsRangeSuffix(granularity)),
          ],
        ),
      );
    }
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: AppSpacing.x4),
      child: Align(alignment: Alignment.centerLeft, child: caption),
    );
  }

  /// 任意の開始日と終了日の入力 (FR-18)。
  Widget _customFields(AppLocalizations l10n) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: AppSpacing.x4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Expanded(
                child: DayField(
                  key: const Key('stats-start-day'),
                  controller: _startController,
                  label: l10n.statsStartLabel,
                  errorText: _startError,
                ),
              ),
              const SizedBox(width: AppSpacing.x3),
              Expanded(
                child: DayField(
                  key: const Key('stats-end-day'),
                  controller: _endController,
                  label: l10n.statsEndLabel,
                  errorText: _endError,
                ),
              ),
            ],
          ),
          const SizedBox(height: AppSpacing.x3),
          Align(
            alignment: Alignment.centerRight,
            child: OutlinedButton(
              onPressed: _applyCustom,
              style: OutlinedButton.styleFrom(
                minimumSize: const Size(0, 36),
                padding: const EdgeInsets.symmetric(horizontal: AppSpacing.x4),
                textStyle: AppTextStyle.label(
                  color: BrewbookTheme.of(context).palette.ink,
                ),
              ),
              child: Text(l10n.statsApplyButton),
            ),
          ),
          if (_startError == null && _endError == null && _customStart != null && _customEnd != null)
            Padding(
              padding: const EdgeInsets.only(top: AppSpacing.x2),
              child: Text(
                l10n.statsCustomGranularityNote(
                  statsDayCount(_customStart!, _customEnd!),
                  _granularityLabel(
                    l10n,
                    statsPeriodFor(
                      StatsPeriodPreset.custom,
                      widget.services.clock.now(),
                      customStart: _customStart,
                      customEnd: _customEnd,
                    ).granularity,
                  ),
                ),
                style: AppTextStyle.caption(color: BrewbookTheme.of(context).palette.inkMuted),
              ),
            ),
        ],
      ),
    );
  }

  /// 集計の結果のグラフ。読み込み中と失敗の表示も担う。
  ///
  /// 広い画面ではグラフを 2 列に並べる (Stats のガイドライン)。
  Widget _body(BuildContext context, AppLocalizations l10n, bool wide) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    if (_loading) {
      return Center(
        child: Text(l10n.loading, style: AppTextStyle.body(color: brewbook.palette.inkMuted)),
      );
    }
    if (_errorMessage != null) {
      return Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(AppSpacing.x6),
          child: ErrorBanner(message: _errorMessage!, onRetry: _load),
        ),
      );
    }
    final Widget brewCharts = BrewStatsCharts(periods: _brews);
    final Widget purchaseCharts = PurchaseStatsCharts(purchases: _purchases);
    final Widget scatterCharts = BrewRatingScatterCharts(ratings: _ratings);
    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(
        AppSpacing.x4,
        AppSpacing.x2,
        AppSpacing.x4,
        AppSpacing.x8,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          _rangeCaption(l10n),
          const SizedBox(height: AppSpacing.x6),
          if (wide) ...<Widget>[
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Expanded(child: brewCharts),
                const SizedBox(width: AppSpacing.x8),
                Expanded(child: purchaseCharts),
              ],
            ),
            const SizedBox(height: AppSpacing.x8),
            scatterCharts,
          ] else ...<Widget>[
            brewCharts,
            const SizedBox(height: AppSpacing.x6),
            purchaseCharts,
            const SizedBox(height: AppSpacing.x6),
            scatterCharts,
          ],
        ],
      ),
    );
  }

  /// 期間の切り替えの文言 (FR-16)。
  String _presetLabel(AppLocalizations l10n, StatsPeriodPreset preset) {
    return switch (preset) {
      StatsPeriodPreset.currentMonth => l10n.statsPeriodCurrentMonth,
      StatsPeriodPreset.threeMonths => l10n.statsPeriodThreeMonths,
      StatsPeriodPreset.sixMonths => l10n.statsPeriodSixMonths,
      StatsPeriodPreset.twelveMonths => l10n.statsPeriodTwelveMonths,
      StatsPeriodPreset.allTime => l10n.statsPeriodAllTime,
      StatsPeriodPreset.custom => l10n.statsPeriodCustom,
    };
  }

  /// 粒度の文言 (日別 / 月別)。
  String _granularityLabel(AppLocalizations l10n, StatsGranularity granularity) {
    return switch (granularity) {
      StatsGranularity.day => l10n.statsGranularityDaily,
      StatsGranularity.month => l10n.statsGranularityMonthly,
    };
  }
}
