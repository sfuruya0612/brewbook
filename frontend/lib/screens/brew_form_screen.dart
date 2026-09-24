import 'package:flutter/material.dart';

import '../api/models.dart';
import '../api/record_inputs.dart';
import '../api/records_api.dart';
import '../l10n/app_localizations.dart';
import '../records/record_services.dart';
import '../records/values.dart';
import '../widgets/day_time_fields.dart';
import '../widgets/error_banner.dart';
import '../widgets/error_message.dart';
import '../widgets/picker_tile.dart';
import '../widgets/record_picker.dart';
import '../widgets/suggestion_field.dart';

/// 抽出の登録と編集の画面 (FR-11)。
///
/// 購入は必須で、ダイアログから選ぶ。抽出日時は端末のローカル時刻で入力し、送信の直前に
/// UTC の ISO 8601 へ変換する (FR-11)。抽出方法と挽き目は、入力中に過去の入力値の候補を出す (FR-13)。
class BrewFormScreen extends StatefulWidget {
  const BrewFormScreen({super.key, required this.services, this.id});

  /// 記録の画面が使う依存 (ADR-0007)。
  final RecordServices services;

  /// 編集する抽出の ID。新規の登録のときは null。
  final String? id;

  @override
  State<BrewFormScreen> createState() => _BrewFormScreenState();
}

class _BrewFormScreenState extends State<BrewFormScreen> {
  final TextEditingController _date = TextEditingController();
  final TextEditingController _time = TextEditingController();
  final TextEditingController _dose = TextEditingController();
  final TextEditingController _water = TextEditingController();
  final TextEditingController _waterTemp = TextEditingController();
  final TextEditingController _brewTime = TextEditingController();
  final TextEditingController _method = TextEditingController();
  final TextEditingController _grindSetting = TextEditingController();
  final TextEditingController _notes = TextEditingController();

  Purchase? _purchase;
  int? _rating;

  String? _purchaseError;
  String? _dateError;
  String? _timeError;
  String? _doseError;
  String? _waterError;
  String? _waterTempError;
  String? _brewTimeError;
  String? _errorMessage;
  String? _loadError;
  bool _busy = false;
  bool _loading = false;

  @override
  void initState() {
    super.initState();
    // 既定値は端末のタイムゾーンでの現在の日時とする (FR-11)。
    final current = now();
    _date.text = formatDay(current);
    _time.text = formatTime(current.hour, current.minute);
    final id = widget.id;
    if (id != null) {
      _load(id);
    }
  }

  @override
  void dispose() {
    _date.dispose();
    _time.dispose();
    _dose.dispose();
    _water.dispose();
    _waterTemp.dispose();
    _brewTime.dispose();
    _method.dispose();
    _grindSetting.dispose();
    _notes.dispose();
    super.dispose();
  }

  /// 編集のために現在の値を読み込む。
  Future<void> _load(String id) async {
    // 再試行で回復できるよう、前回の失敗の表示を消してから読み直す。
    setState(() {
      _loading = true;
      _loadError = null;
    });
    try {
      final brew = await widget.services.records.brew(id);
      if (!mounted) {
        return;
      }
      final local = parseUtcToLocal(brew.brewedAt);
      setState(() {
        _purchase = brew.purchase;
        _date.text = formatDay(local);
        _time.text = formatTime(local.hour, local.minute);
        _dose.text = brew.doseGrams == null ? '' : formatNumber(brew.doseGrams!);
        _water.text = brew.waterGrams == null ? '' : formatNumber(brew.waterGrams!);
        _waterTemp.text = brew.waterTempC == null ? '' : formatNumber(brew.waterTempC!);
        _brewTime.text = brew.brewTimeSeconds?.toString() ?? '';
        _method.text = brew.method ?? '';
        _grindSetting.text = brew.grindSetting ?? '';
        _rating = brew.rating;
        _notes.text = brew.notes ?? '';
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() => _loadError = messageForError(error, AppLocalizations.of(context)));
    } finally {
      if (mounted) {
        setState(() => _loading = false);
      }
    }
  }

  /// 購入を選ばせる (必須。FR-11)。
  Future<void> _pickPurchase() async {
    final l10n = AppLocalizations.of(context);
    final choice = await showRecordPicker<Purchase>(
      context: context,
      title: l10n.selectPurchaseTitle,
      services: widget.services,
      load: ({cursor, required includeArchived}) =>
          widget.services.records.purchases(cursor: cursor, includeArchived: false),
      titleBuilder: (context, purchase) => Text(purchase.product.name),
      subtitleBuilder: (context, purchase) => Text(
        displayDay(purchase.purchasedOn, Localizations.localeOf(context)),
      ),
    );
    if (choice == null || !mounted) {
      return;
    }
    setState(() {
      _purchase = choice.value;
      _purchaseError = null;
    });
  }

  /// 入力した値で登録または更新する。
  Future<void> _save() async {
    final l10n = AppLocalizations.of(context);
    final purchase = _purchase;
    final date = parseDay(_date.text);
    final time = parseTime(_time.text);
    final dose = _decimal(_dose, l10n);
    final water = _decimal(_water, l10n);
    final waterTemp = _decimal(_waterTemp, l10n);
    final brewTime = _count(_brewTime, l10n);

    final purchaseError = purchase == null ? l10n.validationPurchase : null;
    final dateError = date == null ? l10n.validationDay : null;
    final timeError = time == null ? l10n.validationTime : null;
    setState(() {
      _purchaseError = purchaseError;
      _dateError = dateError;
      _timeError = timeError;
      _doseError = dose.error;
      _waterError = water.error;
      _waterTempError = waterTemp.error;
      _brewTimeError = brewTime.error;
      _errorMessage = null;
    });
    if (purchaseError != null ||
        dateError != null ||
        timeError != null ||
        dose.error != null ||
        water.error != null ||
        waterTemp.error != null ||
        brewTime.error != null) {
      return;
    }

    // 端末のローカル時刻の日時を、UTC の ISO 8601 にする (FR-11)。
    final brewedAt = toUtcIso8601(
      DateTime(date!.year, date.month, date.day, time!.hour, time.minute),
    );
    final input = BrewInput(
      purchaseId: purchase!.id,
      brewedAt: brewedAt,
      doseGrams: dose.value,
      waterGrams: water.value,
      waterTempC: waterTemp.value,
      brewTimeSeconds: brewTime.value,
      method: _optionalText(_method),
      grindSetting: _optionalText(_grindSetting),
      rating: _rating,
      notes: _optionalText(_notes),
    );
    setState(() => _busy = true);
    try {
      final id = widget.id;
      if (id == null) {
        await widget.services.records.createBrew(input);
      } else {
        await widget.services.records.updateBrew(id, input);
      }
      widget.services.markRecordsChanged();
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.savedMessage)),
      );
      Navigator.of(context).pop();
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() => _errorMessage = messageForError(error, l10n));
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  /// 小数の入力を読む。空のときは null を返し、読めないときは誤りを持つ (FR-11)。
  static ({double? value, String? error}) _decimal(
    TextEditingController controller,
    AppLocalizations l10n,
  ) {
    final text = controller.text.trim();
    if (text.isEmpty) {
      return (value: null, error: null);
    }
    final value = parseDecimal(text);
    return (value: value, error: value == null ? l10n.validationDecimal : null);
  }

  /// 整数の入力を読む。空のときは null を返し、読めないときは誤りを持つ (FR-11)。
  static ({int? value, String? error}) _count(
    TextEditingController controller,
    AppLocalizations l10n,
  ) {
    final text = controller.text.trim();
    if (text.isEmpty) {
      return (value: null, error: null);
    }
    final value = parseCount(text);
    return (value: value, error: value == null ? l10n.validationNumber : null);
  }

  /// 空の入力を API の null にする。
  static String? _optionalText(TextEditingController controller) {
    final value = controller.text.trim();
    return value.isEmpty ? null : value;
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(
        title: Text(widget.id == null ? l10n.brewNewTitle : l10n.brewEditTitle),
      ),
      body: _body(context, l10n),
    );
  }

  Widget _body(BuildContext context, AppLocalizations l10n) {
    if (_loading) {
      return const Center(child: CircularProgressIndicator());
    }
    final loadError = _loadError;
    if (loadError != null) {
      return Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: ErrorBanner(message: loadError, onRetry: () => _load(widget.id!)),
        ),
      );
    }
    final records = widget.services.records;
    final purchase = _purchase;
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          PickerTile(
            label: l10n.purchaseLabel,
            value: purchase == null
                ? null
                : l10n.purchaseRowSubtitle(
                    displayDay(purchase.purchasedOn, Localizations.localeOf(context)),
                    purchase.product.name,
                  ),
            errorText: _purchaseError,
            onPressed: _busy ? null : _pickPurchase,
          ),
          const SizedBox(height: 16),
          DayField(
            controller: _date,
            label: l10n.brewedAtLabel,
            enabled: !_busy,
            errorText: _dateError,
          ),
          const SizedBox(height: 16),
          TimeField(
            controller: _time,
            label: l10n.brewedAtLabel,
            enabled: !_busy,
            errorText: _timeError,
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _dose,
            enabled: !_busy,
            keyboardType: const TextInputType.numberWithOptions(decimal: true),
            decoration: InputDecoration(
              labelText: l10n.doseLabel,
              errorText: _doseError,
            ),
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _water,
            enabled: !_busy,
            keyboardType: const TextInputType.numberWithOptions(decimal: true),
            decoration: InputDecoration(
              labelText: l10n.waterLabel,
              errorText: _waterError,
            ),
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _waterTemp,
            enabled: !_busy,
            keyboardType: const TextInputType.numberWithOptions(decimal: true),
            decoration: InputDecoration(
              labelText: l10n.waterTempLabel,
              errorText: _waterTempError,
            ),
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _brewTime,
            enabled: !_busy,
            keyboardType: TextInputType.number,
            decoration: InputDecoration(
              labelText: l10n.brewTimeLabel,
              errorText: _brewTimeError,
            ),
          ),
          const SizedBox(height: 16),
          SuggestionField(
            records: records,
            controller: _method,
            field: SuggestionFields.method,
            label: l10n.methodLabel,
            enabled: !_busy,
          ),
          const SizedBox(height: 16),
          SuggestionField(
            records: records,
            controller: _grindSetting,
            field: SuggestionFields.grindSetting,
            label: l10n.grindSettingLabel,
            enabled: !_busy,
          ),
          const SizedBox(height: 16),
          DropdownButtonFormField<int?>(
            initialValue: _rating,
            decoration: InputDecoration(labelText: l10n.ratingLabel),
            items: <DropdownMenuItem<int?>>[
              DropdownMenuItem<int?>(value: null, child: Text(l10n.unsetLabel)),
              for (int value = 1; value <= 5; value++)
                DropdownMenuItem<int?>(value: value, child: Text(value.toString())),
            ],
            onChanged: _busy ? null : (value) => setState(() => _rating = value),
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _notes,
            enabled: !_busy,
            maxLines: 3,
            decoration: InputDecoration(labelText: l10n.notesLabel),
          ),
          const SizedBox(height: 24),
          FilledButton(
            onPressed: _busy ? null : _save,
            child: Text(l10n.saveButton),
          ),
          if (_busy) ...<Widget>[
            const SizedBox(height: 16),
            const Center(child: CircularProgressIndicator()),
          ],
          if (_errorMessage != null) ...<Widget>[
            const SizedBox(height: 16),
            ErrorBanner(message: _errorMessage!),
          ],
        ],
      ),
    );
  }
}
