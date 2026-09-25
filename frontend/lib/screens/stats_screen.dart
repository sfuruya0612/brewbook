import 'package:flutter/material.dart';

import '../api/models.dart';
import '../l10n/app_localizations.dart';
import '../records/record_services.dart';
import '../records/stats_period.dart';
import '../records/values.dart';
import '../widgets/day_time_fields.dart';
import '../widgets/error_banner.dart';
import '../widgets/error_message.dart';
import '../widgets/stats_charts.dart';

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
      body: Column(
        children: <Widget>[
          _periodSelector(l10n),
          if (_preset == StatsPeriodPreset.custom) _customFields(l10n),
          Expanded(child: _body(context, l10n)),
        ],
      ),
    );
  }

  /// 期間の切り替え (FR-18)。
  Widget _periodSelector(AppLocalizations l10n) {
    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      child: Row(
        children: <Widget>[
          for (final preset in StatsPeriodPreset.values)
            Padding(
              padding: const EdgeInsets.only(right: 8),
              child: ChoiceChip(
                key: Key('stats-period-${preset.name}'),
                label: Text(_presetLabel(l10n, preset)),
                selected: _preset == preset,
                onSelected: (selected) => _selectPreset(preset),
              ),
            ),
        ],
      ),
    );
  }

  /// 任意の開始日と終了日の入力 (FR-18)。
  Widget _customFields(AppLocalizations l10n) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Row(
            children: <Widget>[
              Expanded(
                child: DayField(
                  key: const Key('stats-start-day'),
                  controller: _startController,
                  label: l10n.statsStartLabel,
                  errorText: _startError,
                ),
              ),
              const SizedBox(width: 8),
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
          Align(
            alignment: Alignment.centerRight,
            child: Padding(
              padding: const EdgeInsets.only(top: 8),
              child: FilledButton(
                onPressed: _applyCustom,
                child: Text(l10n.statsApplyButton),
              ),
            ),
          ),
        ],
      ),
    );
  }

  /// 集計の結果のグラフ。読み込み中と失敗の表示も担う。
  Widget _body(BuildContext context, AppLocalizations l10n) {
    if (_loading) {
      return const Center(child: CircularProgressIndicator());
    }
    if (_errorMessage != null) {
      return Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: ErrorBanner(message: _errorMessage!, onRetry: _load),
        ),
      );
    }
    return SingleChildScrollView(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          BrewStatsCharts(periods: _brews),
          PurchaseStatsCharts(purchases: _purchases),
          BrewRatingScatterCharts(ratings: _ratings),
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
}
