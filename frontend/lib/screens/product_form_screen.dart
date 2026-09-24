import 'package:flutter/material.dart';

import '../api/record_inputs.dart';
import '../api/records_api.dart';
import '../l10n/app_localizations.dart';
import '../records/record_services.dart';
import '../widgets/error_banner.dart';
import '../widgets/error_message.dart';
import '../widgets/suggestion_field.dart';

/// 商品の登録と編集の画面 (FR-7、FR-8)。
///
/// 商品名は必須。Producer、Origin、Region、Process、Variety は自由記述で、入力中に過去の
/// 入力値の候補を出す (FR-13)。Flavor Notes はタグとして追加と削除ができ、更新では入力した
/// 配列で置き換える (FR-8)。
class ProductFormScreen extends StatefulWidget {
  const ProductFormScreen({super.key, required this.services, this.id});

  /// 記録の画面が使う依存 (ADR-0007)。
  final RecordServices services;

  /// 編集する商品の ID。新規の登録のときは null。
  final String? id;

  @override
  State<ProductFormScreen> createState() => _ProductFormScreenState();
}

class _ProductFormScreenState extends State<ProductFormScreen> {
  final TextEditingController _name = TextEditingController();
  final TextEditingController _producer = TextEditingController();
  final TextEditingController _origin = TextEditingController();
  final TextEditingController _region = TextEditingController();
  final TextEditingController _process = TextEditingController();
  final TextEditingController _variety = TextEditingController();
  final TextEditingController _tag = TextEditingController();

  final List<String> _flavorNotes = <String>[];

  String? _nameError;
  String? _errorMessage;
  String? _loadError;
  bool _busy = false;
  bool _loading = false;

  @override
  void initState() {
    super.initState();
    final id = widget.id;
    if (id != null) {
      _load(id);
    }
  }

  @override
  void dispose() {
    _name.dispose();
    _producer.dispose();
    _origin.dispose();
    _region.dispose();
    _process.dispose();
    _variety.dispose();
    _tag.dispose();
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
      final product = await widget.services.records.product(id);
      if (!mounted) {
        return;
      }
      setState(() {
        _name.text = product.name;
        _producer.text = product.producer ?? '';
        _origin.text = product.origin ?? '';
        _region.text = product.region ?? '';
        _process.text = product.process ?? '';
        _variety.text = product.variety ?? '';
        _flavorNotes
          ..clear()
          ..addAll(product.flavorNotes);
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

  /// タグを 1 つ足す。前後の空白を除いた名前が空のときは何もしない (FR-8)。
  void _addTag() {
    final tag = _tag.text.trim();
    if (tag.isEmpty) {
      return;
    }
    setState(() {
      if (!_flavorNotes.contains(tag)) {
        _flavorNotes.add(tag);
      }
      _tag.clear();
    });
  }

  /// 入力した値で登録または更新する。
  Future<void> _save() async {
    final l10n = AppLocalizations.of(context);
    final name = _name.text.trim();
    final nameError = name.isEmpty ? l10n.validationRequired : null;
    setState(() {
      _nameError = nameError;
      _errorMessage = null;
    });
    if (nameError != null) {
      return;
    }
    final input = ProductInput(
      name: name,
      producer: _optional(_producer),
      origin: _optional(_origin),
      region: _optional(_region),
      process: _optional(_process),
      variety: _optional(_variety),
      flavorNotes: List<String>.of(_flavorNotes),
    );
    setState(() => _busy = true);
    try {
      final id = widget.id;
      if (id == null) {
        await widget.services.records.createProduct(input);
      } else {
        await widget.services.records.updateProduct(id, input);
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

  /// 空の入力を API の null にする。
  static String? _optional(TextEditingController controller) {
    final value = controller.text.trim();
    return value.isEmpty ? null : value;
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(
        title: Text(widget.id == null ? l10n.productNewTitle : l10n.productEditTitle),
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
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          TextField(
            controller: _name,
            enabled: !_busy,
            decoration: InputDecoration(
              labelText: l10n.productNameLabel,
              errorText: _nameError,
            ),
          ),
          const SizedBox(height: 16),
          SuggestionField(
            records: records,
            controller: _producer,
            field: SuggestionFields.producer,
            label: l10n.producer,
            enabled: !_busy,
          ),
          const SizedBox(height: 16),
          SuggestionField(
            records: records,
            controller: _origin,
            field: SuggestionFields.origin,
            label: l10n.origin,
            enabled: !_busy,
          ),
          const SizedBox(height: 16),
          SuggestionField(
            records: records,
            controller: _region,
            field: SuggestionFields.region,
            label: l10n.region,
            enabled: !_busy,
          ),
          const SizedBox(height: 16),
          SuggestionField(
            records: records,
            controller: _process,
            field: SuggestionFields.process,
            label: l10n.process,
            enabled: !_busy,
          ),
          const SizedBox(height: 16),
          SuggestionField(
            records: records,
            controller: _variety,
            field: SuggestionFields.variety,
            label: l10n.variety,
            enabled: !_busy,
          ),
          const SizedBox(height: 24),
          Text(l10n.flavorNotes, style: Theme.of(context).textTheme.titleMedium),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            children: <Widget>[
              for (final note in _flavorNotes)
                InputChip(
                  label: Text(note),
                  deleteButtonTooltipMessage: l10n.deleteButton,
                  onDeleted: _busy ? null : () => setState(() => _flavorNotes.remove(note)),
                ),
            ],
          ),
          const SizedBox(height: 8),
          Row(
            children: <Widget>[
              Expanded(
                child: TextField(
                  controller: _tag,
                  enabled: !_busy,
                  decoration: InputDecoration(
                    labelText: l10n.tagInputLabel,
                    hintText: l10n.tagInputHint,
                  ),
                  onSubmitted: (_) => _addTag(),
                ),
              ),
              const SizedBox(width: 8),
              FilledButton.tonal(
                onPressed: _busy ? null : _addTag,
                child: Text(l10n.addButton),
              ),
            ],
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
