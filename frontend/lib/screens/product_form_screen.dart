import 'dart:ui' show PathMetric;

import 'package:flutter/material.dart';

import '../api/models.dart';
import '../api/record_inputs.dart';
import '../api/records_api.dart';
import '../l10n/app_localizations.dart';
import '../records/record_services.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';
import '../widgets/app_field.dart';
import '../widgets/app_form.dart';
import '../widgets/error_banner.dart';
import '../widgets/error_message.dart';
import '../widgets/suggestion_field.dart';

/// 商品の登録と編集の画面 (FR-7、FR-8)。
///
/// 商品名は必須。Producer、Origin、Region、Process、Variety は自由記述で、入力中に過去の
/// 入力値の候補を出す (FR-13)。Flavor Notes はタグとして追加と削除ができ、更新では入力した
/// 配列で置き換える (FR-8)。
/// 写真からの推測 (FR-19) の導線は、推測した値を [initial] に渡して開き、保存した商品を
/// [onSaved] で返せるようにする。
class ProductFormScreen extends StatefulWidget {
  const ProductFormScreen({
    super.key,
    required this.services,
    this.id,
    this.embedded = false,
    this.initial,
    this.onClose,
    this.onSaved,
  });

  /// 記録の画面が使う依存 (ADR-0007)。
  final RecordServices services;

  /// 編集する商品の ID。新規の登録のときは null。
  final String? id;

  /// 幅 840 px 以上の 2 段組の右の面に出すか。
  final bool embedded;

  /// 新規の登録のときに最初から入れる値 (FR-19 の推測の引き継ぎ)。編集では使わない。
  final ProductInput? initial;

  /// 閉じる動き。無いときは前の画面へ戻る。
  final VoidCallback? onClose;

  /// 保存できたときの動き。保存した商品を受け取る。無いときは前の画面へ戻る。
  ///
  /// 押し出しの画面へ結果を返すとき (FR-19 の商品の登録の導線) は、この画面では保存の通知を
  /// 出さない。2 段組では画面の下の面にも Scaffold があり、同じ SnackBar が 2 つあると
  /// みなされて戻る遷移が失敗するためである。この場合の通知は購入の保存のときに出す。
  final ValueChanged<Product>? onSaved;

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

  final List<String> _flavorNotes = <String>[];

  String? _nameError;
  String? _errorMessage;
  String? _loadError;

  /// 検証の誤りを上のバナーで示すか。
  bool _showValidationBanner = false;
  bool _busy = false;
  bool _loading = false;

  /// アーカイブ済みか (編集のときだけ使う。FR-12)。
  bool _archived = false;

  @override
  void initState() {
    super.initState();
    final id = widget.id;
    if (id != null) {
      _load(id);
      return;
    }
    // 写真からの推測 (FR-19) の値を引き継いで開いたときは、最初から入れておく。
    final initial = widget.initial;
    if (initial != null) {
      _name.text = initial.name;
      _producer.text = initial.producer ?? '';
      _origin.text = initial.origin ?? '';
      _region.text = initial.region ?? '';
      _process.text = initial.process ?? '';
      _variety.text = initial.variety ?? '';
      _flavorNotes.addAll(initial.flavorNotes);
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
        _archived = product.isArchived;
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
  void _addTag(String value) {
    final tag = value.trim();
    if (tag.isEmpty) {
      return;
    }
    setState(() {
      if (!_flavorNotes.contains(tag)) {
        _flavorNotes.add(tag);
      }
    });
  }

  /// タグの入力をダイアログで受ける (docs/design/components/Chip)。
  Future<void> _promptTag() async {
    final l10n = AppLocalizations.of(context);
    final tag = await showDialog<String>(
      context: context,
      builder: (context) => _TagDialog(
        title: l10n.tagInputLabel,
        hintText: l10n.tagInputHint,
        confirmLabel: l10n.addButton,
        cancelLabel: l10n.cancelButton,
      ),
    );
    if (tag != null) {
      _addTag(tag);
    }
  }

  /// アーカイブとアーカイブ解除を行う (FR-12)。
  Future<void> _toggleArchived() async {
    final id = widget.id;
    if (id == null) {
      return;
    }
    final l10n = AppLocalizations.of(context);
    try {
      final updated = await widget.services.records.setProductArchived(id, !_archived);
      widget.services.markRecordsChanged();
      if (!mounted) {
        return;
      }
      setState(() => _archived = updated.isArchived);
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(updated.isArchived ? l10n.archivedMessage : l10n.unarchivedMessage),
        ),
      );
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(messageForError(error, l10n))),
      );
    }
  }

  /// 入力した値で登録または更新する。
  Future<void> _save() async {
    final l10n = AppLocalizations.of(context);
    final name = _name.text.trim();
    final nameError = name.isEmpty ? l10n.validationRequired : null;
    setState(() {
      _nameError = nameError;
      _showValidationBanner = nameError != null;
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
      final Product saved = id == null
          ? await widget.services.records.createProduct(input)
          : await widget.services.records.updateProduct(id, input);
      widget.services.markRecordsChanged();
      if (!mounted) {
        return;
      }
      // 保存した商品を呼び出し元に返す (FR-19 の商品の登録の導線)。無いときは前の画面へ戻る。
      // 通知は、押し出しの画面へ結果を返すとき以外はここで出す (パラメータの説明を参照)。
      final onSaved = widget.onSaved;
      if (onSaved != null) {
        onSaved(saved);
        if (widget.embedded) {
          _showSaved(l10n);
        }
      } else {
        Navigator.of(context).pop();
        _showSaved(l10n);
      }
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

  /// 保存できたことを知らせる (FR-8)。
  void _showSaved(AppLocalizations l10n) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(l10n.savedMessage)),
    );
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
        leading: CloseButton(
          onPressed: widget.onClose ?? () => Navigator.of(context).maybePop(),
        ),
        title: Text(widget.id == null ? l10n.productNewTitle : l10n.productEditTitle),
        actions: <Widget>[
          if (widget.id != null)
            IconButton(
              tooltip: _archived ? l10n.unarchiveButton : l10n.archiveButton,
              icon: Icon(_archived ? Icons.unarchive_outlined : Icons.archive_outlined),
              onPressed: _busy ? null : _toggleArchived,
            ),
          TextButton(
            onPressed: _busy ? null : _save,
            child: Text(l10n.saveButton),
          ),
          const SizedBox(width: AppSpacing.x2),
        ],
      ),
      body: _body(context, l10n),
    );
  }

  Widget _body(BuildContext context, AppLocalizations l10n) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    if (_loading) {
      return Center(
        child: Text(l10n.loading, style: AppTextStyle.body(color: brewbook.palette.inkMuted)),
      );
    }
    final loadError = _loadError;
    if (loadError != null) {
      return Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(AppSpacing.x6),
          child: ErrorBanner(message: loadError, onRetry: () => _load(widget.id!)),
        ),
      );
    }
    final records = widget.services.records;
    return AppForm(
      children: <Widget>[
        if (_showValidationBanner) ErrorBanner(message: l10n.errorValidation),
        AppTextField(
          controller: _name,
          label: l10n.productNameLabel,
          required: true,
          enabled: !_busy,
          errorText: _nameError,
          mono: false,
        ),
        SuggestionField(
          records: records,
          controller: _producer,
          field: SuggestionFields.producer,
          label: l10n.producer,
          enabled: !_busy,
        ),
        AppFormRow(
          children: <Widget>[
            SuggestionField(
              records: records,
              controller: _origin,
              field: SuggestionFields.origin,
              label: l10n.origin,
              enabled: !_busy,
            ),
            SuggestionField(
              records: records,
              controller: _region,
              field: SuggestionFields.region,
              label: l10n.region,
              enabled: !_busy,
            ),
          ],
        ),
        AppFormRow(
          children: <Widget>[
            SuggestionField(
              records: records,
              controller: _process,
              field: SuggestionFields.process,
              label: l10n.process,
              enabled: !_busy,
            ),
            SuggestionField(
              records: records,
              controller: _variety,
              field: SuggestionFields.variety,
              label: l10n.variety,
              enabled: !_busy,
            ),
          ],
        ),
        _flavorNotesField(l10n),
        if (_errorMessage != null) ErrorBanner(message: _errorMessage!),
      ],
    );
  }

  /// フレーバーノートのタグの入力 (FR-8)。
  Widget _flavorNotesField(AppLocalizations l10n) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Row(
          children: <Widget>[
            Text(l10n.flavorNotes, style: AppTextStyle.label(color: brewbook.palette.inkMuted)),
          ],
        ),
        const SizedBox(height: 6),
        Wrap(
          spacing: AppSpacing.x2,
          runSpacing: AppSpacing.x2,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: <Widget>[
            for (final note in _flavorNotes)
              InputChip(
                label: Text(note),
                labelStyle: AppTextStyle.label(color: brewbook.palette.ink),
                backgroundColor: brewbook.cremaSoft,
                side: BorderSide.none,
                shape: const StadiumBorder(),
                deleteIcon: Icon(Icons.close_outlined, size: 14, color: brewbook.palette.inkMuted),
                deleteButtonTooltipMessage: l10n.deleteButton,
                onDeleted: _busy ? null : () => setState(() => _flavorNotes.remove(note)),
              ),
            _TagInputChip(
              label: l10n.tagInputHint,
              enabled: !_busy,
              onTap: _promptTag,
            ),
          ],
        ),
      ],
    );
  }
}

/// 破線の枠の「タグを入力」のチップ (docs/design/components/Chip)。
class _TagInputChip extends StatelessWidget {
  const _TagInputChip({required this.label, required this.enabled, required this.onTap});

  final String label;
  final bool enabled;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    return Material(
      color: Colors.transparent,
      child: InkWell(
        onTap: enabled ? onTap : null,
        borderRadius: AppRadius.fullAll,
        child: CustomPaint(
          painter: _DashedBorderPainter(color: brewbook.lineStrong),
          child: Padding(
            padding: const EdgeInsets.symmetric(
              horizontal: AppSpacing.x3,
              vertical: 6,
            ),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                Icon(Icons.add_outlined, size: 14, color: brewbook.palette.inkMuted),
                const SizedBox(width: 6),
                Text(label, style: AppTextStyle.label(color: brewbook.palette.inkMuted)),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// 破線の角丸の枠を描く。
class _DashedBorderPainter extends CustomPainter {
  const _DashedBorderPainter({required this.color});

  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final paint = Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1;
    final Path path = Path()
      ..addRRect(RRect.fromRectAndRadius(Offset.zero & size, const Radius.circular(AppRadius.full)));
    for (final PathMetric metric in path.computeMetrics()) {
      double distance = 0;
      while (distance < metric.length) {
        final double end = distance + 4;
        canvas.drawPath(metric.extractPath(distance, end), paint);
        distance = end + 4;
      }
    }
  }

  @override
  bool shouldRepaint(covariant _DashedBorderPainter oldDelegate) => oldDelegate.color != color;
}

/// タグの名前を入力させるダイアログ (FR-8)。
class _TagDialog extends StatefulWidget {
  const _TagDialog({
    required this.title,
    required this.hintText,
    required this.confirmLabel,
    required this.cancelLabel,
  });

  final String title;
  final String hintText;
  final String confirmLabel;
  final String cancelLabel;

  @override
  State<_TagDialog> createState() => _TagDialogState();
}

class _TagDialogState extends State<_TagDialog> {
  final TextEditingController _tag = TextEditingController();

  @override
  void dispose() {
    _tag.dispose();
    super.dispose();
  }

  /// 空白を除いたタグを返して閉じる。空のときは閉じない。
  void _confirm() {
    final tag = _tag.text.trim();
    if (tag.isEmpty) {
      return;
    }
    Navigator.of(context).pop(tag);
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: Text(widget.title),
      content: AppTextField(
        controller: _tag,
        label: widget.title,
        hintText: widget.hintText,
        autofocus: true,
        mono: false,
        onSubmitted: (_) => _confirm(),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(widget.cancelLabel),
        ),
        FilledButton(
          onPressed: _confirm,
          child: Text(widget.confirmLabel),
        ),
      ],
    );
  }
}
