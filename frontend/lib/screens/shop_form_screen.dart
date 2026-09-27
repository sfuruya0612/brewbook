import 'package:flutter/material.dart';

import '../api/record_inputs.dart';
import '../l10n/app_localizations.dart';
import '../records/record_services.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';
import '../widgets/app_field.dart';
import '../widgets/app_form.dart';
import '../widgets/error_banner.dart';
import '../widgets/error_message.dart';

/// 店の登録と編集の画面 (FR-6)。
///
/// 店名は必須、住所は任意。編集では現在の値を読み込んでから上書きする。
class ShopFormScreen extends StatefulWidget {
  const ShopFormScreen({
    super.key,
    required this.services,
    this.id,
    this.embedded = false,
    this.onClose,
    this.onSaved,
  });

  /// 記録の画面が使う依存 (ADR-0007)。
  final RecordServices services;

  /// 編集する店の ID。新規の登録のときは null。
  final String? id;

  /// 幅 840 px 以上の 2 段組の右の面に出すか。
  final bool embedded;

  /// 閉じる動き。無いときは前の画面へ戻る。
  final VoidCallback? onClose;

  /// 保存できたときの動き。無いときは前の画面へ戻る。
  final VoidCallback? onSaved;

  @override
  State<ShopFormScreen> createState() => _ShopFormScreenState();
}

class _ShopFormScreenState extends State<ShopFormScreen> {
  final TextEditingController _name = TextEditingController();
  final TextEditingController _address = TextEditingController();

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
    }
  }

  @override
  void dispose() {
    _name.dispose();
    _address.dispose();
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
      final shop = await widget.services.records.shop(id);
      if (!mounted) {
        return;
      }
      setState(() {
        _name.text = shop.name;
        _address.text = shop.address ?? '';
        _archived = shop.isArchived;
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

  /// アーカイブとアーカイブ解除を行う (FR-12)。
  Future<void> _toggleArchived() async {
    final id = widget.id;
    if (id == null) {
      return;
    }
    final l10n = AppLocalizations.of(context);
    try {
      final updated = await widget.services.records.setShopArchived(id, !_archived);
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
    final address = _address.text.trim();
    final input = ShopInput(name: name, address: address.isEmpty ? null : address);
    setState(() => _busy = true);
    try {
      final id = widget.id;
      if (id == null) {
        await widget.services.records.createShop(input);
      } else {
        await widget.services.records.updateShop(id, input);
      }
      widget.services.markRecordsChanged();
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.savedMessage)),
      );
      if (widget.embedded) {
        widget.onSaved?.call();
      } else {
        Navigator.of(context).pop();
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

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(
        leading: CloseButton(
          onPressed: widget.onClose ?? () => Navigator.of(context).maybePop(),
        ),
        title: Text(widget.id == null ? l10n.shopNewTitle : l10n.shopEditTitle),
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
          child: ErrorBanner(
            message: loadError,
            onRetry: () => _load(widget.id!),
          ),
        ),
      );
    }
    return AppForm(
      children: <Widget>[
        if (_showValidationBanner) ErrorBanner(message: l10n.errorValidation),
        AppTextField(
          controller: _name,
          label: l10n.shopNameLabel,
          required: true,
          enabled: !_busy,
          errorText: _nameError,
          mono: false,
        ),
        AppTextField(
          controller: _address,
          label: l10n.addressLabel,
          enabled: !_busy,
          mono: false,
        ),
        if (_errorMessage != null) ErrorBanner(message: _errorMessage!),
      ],
    );
  }
}
