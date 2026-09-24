import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import '../records/values.dart';

/// 日付 (`YYYY-MM-DD`) の入力欄 (FR-9)。
///
/// 既定値は端末のタイムゾーンでの当日とし、文字での入力と、カレンダーからの選択の両方を
/// 受け付ける。日付は API と同じ `YYYY-MM-DD` のまま扱い、タイムゾーンの変換は行わない。
class DayField extends StatelessWidget {
  const DayField({
    super.key,
    required this.controller,
    required this.label,
    this.enabled = true,
    this.errorText,
  });

  final TextEditingController controller;
  final String label;
  final bool enabled;
  final String? errorText;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return TextField(
      controller: controller,
      enabled: enabled,
      keyboardType: TextInputType.datetime,
      decoration: InputDecoration(
        labelText: label,
        hintText: l10n.dayFormatHint,
        errorText: errorText,
        suffixIcon: IconButton(
          tooltip: l10n.selectButton,
          icon: const Icon(Icons.calendar_today_outlined),
          onPressed: enabled ? () => _pick(context) : null,
        ),
      ),
    );
  }

  /// カレンダーから日付を選ばせる。
  ///
  /// カレンダーの範囲は 2000 年から 2100 年とする。入力の検証は 4 桁の年なら
  /// 範囲外も通すため、初期値は範囲に収めてから渡す (範囲外だと `showDatePicker` の
  /// 前提を満たさない)。
  Future<void> _pick(BuildContext context) async {
    final first = DateTime(2000);
    final last = DateTime(2100);
    final initial = parseDay(controller.text) ?? today();
    final picked = await showDatePicker(
      context: context,
      initialDate: initial.isBefore(first)
          ? first
          : initial.isAfter(last)
          ? last
          : initial,
      firstDate: first,
      lastDate: last,
    );
    if (picked != null) {
      controller.text = formatDay(picked);
    }
  }
}

/// 時刻 (`HH:MM`) の入力欄 (FR-11)。
///
/// 端末のローカル時刻で入力させ、送信の直前に UTC へ変換する。
class TimeField extends StatelessWidget {
  const TimeField({
    super.key,
    required this.controller,
    required this.label,
    this.enabled = true,
    this.errorText,
  });

  final TextEditingController controller;
  final String label;
  final bool enabled;
  final String? errorText;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return TextField(
      controller: controller,
      enabled: enabled,
      keyboardType: TextInputType.datetime,
      decoration: InputDecoration(
        labelText: label,
        hintText: l10n.timeFormatHint,
        errorText: errorText,
        suffixIcon: IconButton(
          tooltip: l10n.selectButton,
          icon: const Icon(Icons.schedule_outlined),
          onPressed: enabled ? () => _pick(context) : null,
        ),
      ),
    );
  }

  /// 時計から時刻を選ばせる。
  Future<void> _pick(BuildContext context) async {
    final parsed = parseTime(controller.text);
    final picked = await showTimePicker(
      context: context,
      initialTime: parsed == null
          ? TimeOfDay.now()
          : TimeOfDay(hour: parsed.hour, minute: parsed.minute),
    );
    if (picked != null) {
      controller.text = formatTime(picked.hour, picked.minute);
    }
  }
}
