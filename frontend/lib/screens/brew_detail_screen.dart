import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../api/models.dart';
import '../l10n/app_localizations.dart';
import '../records/record_services.dart';
import '../records/values.dart';
import '../router/app_router.dart';
import '../widgets/detail_row.dart';
import '../widgets/error_banner.dart';
import '../widgets/error_message.dart';

/// 抽出の詳細の画面 (FR-11、UC-6)。
///
/// 使った購入をたどり、その中の商品と店 (登録している場合) もたどれるようにする。
class BrewDetailScreen extends StatefulWidget {
  const BrewDetailScreen({super.key, required this.services, required this.id});

  /// 記録の画面が使う依存 (ADR-0007)。
  final RecordServices services;

  /// 表示する抽出の ID。
  final String id;

  @override
  State<BrewDetailScreen> createState() => _BrewDetailScreenState();
}

class _BrewDetailScreenState extends State<BrewDetailScreen> {
  Brew? _brew;
  String? _errorMessage;
  bool _loading = false;

  @override
  void initState() {
    super.initState();
    widget.services.revision.addListener(_reload);
    _load();
  }

  @override
  void dispose() {
    widget.services.revision.removeListener(_reload);
    super.dispose();
  }

  /// 記録が変わったので読み直す (編集やアーカイブの後)。
  void _reload() {
    if (mounted) {
      _load();
    }
  }

  /// 抽出を読み込む。アーカイブ済みでも返る (FR-12)。
  Future<void> _load() async {
    setState(() {
      _loading = true;
      _errorMessage = null;
    });
    try {
      final brew = await widget.services.records.brew(widget.id);
      if (!mounted) {
        return;
      }
      setState(() => _brew = brew);
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() => _errorMessage = messageForError(error, AppLocalizations.of(context)));
    } finally {
      if (mounted) {
        setState(() => _loading = false);
      }
    }
  }

  /// アーカイブとアーカイブ解除を行う (FR-12)。
  Future<void> _toggleArchived() async {
    final brew = _brew;
    if (brew == null) {
      return;
    }
    final l10n = AppLocalizations.of(context);
    try {
      // 応答の行で画面の状態を更新する (アーカイブの切り替えを画面に反映する)。
      final updated = await widget.services.records.setBrewArchived(brew.id, !brew.isArchived);
      widget.services.markRecordsChanged();
      if (!mounted) {
        return;
      }
      setState(() => _brew = updated);
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

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final brew = _brew;
    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.brewDetailTitle),
        actions: <Widget>[
          if (brew != null) ...<Widget>[
            IconButton(
              tooltip: brew.isArchived ? l10n.unarchiveButton : l10n.archiveButton,
              icon: Icon(brew.isArchived ? Icons.unarchive_outlined : Icons.archive_outlined),
              onPressed: _toggleArchived,
            ),
            IconButton(
              tooltip: l10n.editButton,
              icon: const Icon(Icons.edit_outlined),
              onPressed: () => context.push(AppRoutes.brewEditPath(widget.id)),
            ),
          ],
        ],
      ),
      body: _body(context, l10n, brew),
    );
  }

  Widget _body(BuildContext context, AppLocalizations l10n, Brew? brew) {
    if (brew == null) {
      if (_loading) {
        return const Center(child: CircularProgressIndicator());
      }
      return Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: ErrorBanner(
            message: _errorMessage ?? l10n.errorUnexpected,
            onRetry: _load,
          ),
        ),
      );
    }
    final locale = Localizations.localeOf(context);
    final purchase = brew.purchase;
    final shop = purchase.shop;
    return SingleChildScrollView(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          DetailRow(
            label: l10n.brewedAtLabel,
            value: displayTimestamp(brew.brewedAt, locale),
          ),
          DetailRow(
            label: l10n.purchaseLabel,
            value: displayDay(purchase.purchasedOn, locale),
            onTap: () => context.push(AppRoutes.purchasePath(purchase.id)),
          ),
          DetailRow(
            label: l10n.productLabel,
            value: purchase.product.name,
            onTap: () => context.push(AppRoutes.productEditPath(purchase.product.id)),
          ),
          DetailRow(
            label: l10n.shopLabel,
            value: shop?.name ?? l10n.unsetLabel,
            onTap: shop == null ? null : () => context.push(AppRoutes.shopEditPath(shop.id)),
          ),
          DetailRow(label: l10n.doseLabel, value: _grams(l10n, brew.doseGrams)),
          DetailRow(label: l10n.waterLabel, value: _grams(l10n, brew.waterGrams)),
          DetailRow(
            label: l10n.waterTempLabel,
            value: brew.waterTempC == null
                ? l10n.unsetLabel
                : l10n.celsiusValue(formatNumber(brew.waterTempC!)),
          ),
          DetailRow(
            label: l10n.brewTimeLabel,
            value: brew.brewTimeSeconds == null
                ? l10n.unsetLabel
                : l10n.secondsValue(brew.brewTimeSeconds!.toString()),
          ),
          DetailRow(label: l10n.methodLabel, value: valueOrUnset(brew.method, l10n)),
          DetailRow(label: l10n.grindSettingLabel, value: valueOrUnset(brew.grindSetting, l10n)),
          DetailRow(
            label: l10n.ratingLabel,
            value: brew.rating == null
                ? l10n.unsetLabel
                : l10n.ratingValue(brew.rating!.toString()),
          ),
          DetailRow(label: l10n.notesLabel, value: valueOrUnset(brew.notes, l10n)),
          if (_errorMessage != null) ...<Widget>[
            const SizedBox(height: 16),
            ErrorBanner(message: _errorMessage!),
          ],
        ],
      ),
    );
  }

  /// 豆の量や湯量をグラムで表示する。
  String _grams(AppLocalizations l10n, double? value) {
    if (value == null) {
      return l10n.unsetLabel;
    }
    return l10n.gramsValue(formatNumber(value));
  }
}
