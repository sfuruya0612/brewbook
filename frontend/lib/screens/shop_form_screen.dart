import 'package:flutter/material.dart';

import '../api/record_inputs.dart';
import '../l10n/app_localizations.dart';
import '../records/record_services.dart';
import '../widgets/error_banner.dart';
import '../widgets/error_message.dart';

/// 店の登録と編集の画面 (FR-6)。
///
/// 店名は必須、住所は任意。編集では現在の値を読み込んでから上書きする。
class ShopFormScreen extends StatefulWidget {
  const ShopFormScreen({super.key, required this.services, this.id});

  /// 記録の画面が使う依存 (ADR-0007)。
  final RecordServices services;

  /// 編集する店の ID。新規の登録のときは null。
  final String? id;

  @override
  State<ShopFormScreen> createState() => _ShopFormScreenState();
}

class _ShopFormScreenState extends State<ShopFormScreen> {
  final TextEditingController _name = TextEditingController();
  final TextEditingController _address = TextEditingController();

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

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(title: Text(widget.id == null ? l10n.shopNewTitle : l10n.shopEditTitle)),
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
          child: ErrorBanner(
            message: loadError,
            onRetry: () => _load(widget.id!),
          ),
        ),
      );
    }
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          TextField(
            controller: _name,
            enabled: !_busy,
            decoration: InputDecoration(
              labelText: l10n.shopNameLabel,
              errorText: _nameError,
            ),
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _address,
            enabled: !_busy,
            decoration: InputDecoration(labelText: l10n.addressLabel),
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
