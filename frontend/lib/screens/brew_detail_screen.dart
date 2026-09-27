import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../api/models.dart';
import '../l10n/app_localizations.dart';
import '../records/record_services.dart';
import '../records/values.dart';
import '../router/app_router.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';
import '../widgets/app_form.dart';
import '../widgets/error_banner.dart';
import '../widgets/error_message.dart';
import '../widgets/ledger.dart';
import '../widgets/list_row.dart';
import '../widgets/rating.dart';
import '../widgets/reference_tile.dart';

/// 抽出の詳細の画面 (FR-11、UC-6)。
///
/// 使った購入をたどり、その中の商品と店 (登録している場合) もたどれるようにする。
/// 幅 840 px 以上の 2 段組では [embedded] を true にし、右の面に出す。
class BrewDetailScreen extends StatefulWidget {
  const BrewDetailScreen({
    super.key,
    required this.services,
    required this.id,
    this.embedded = false,
    this.onEdit,
  });

  /// 記録の画面が使う依存 (ADR-0007)。
  final RecordServices services;

  /// 表示する抽出の ID。
  final String id;

  /// 2 段組の右の面に出すか。戻るの代わりに閉じる操作を置かない。
  final bool embedded;

  /// 編集を開く動き。無いときは編集の経路を上に積む。
  final VoidCallback? onEdit;

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
        automaticallyImplyLeading: !widget.embedded,
        actions: <Widget>[
          if (brew != null) ...<Widget>[
            IconButton(
              tooltip: brew.isArchived ? l10n.unarchiveButton : l10n.archiveButton,
              icon: Icon(
                brew.isArchived ? Icons.unarchive_outlined : Icons.archive_outlined,
              ),
              onPressed: _toggleArchived,
            ),
            IconButton(
              tooltip: l10n.editButton,
              icon: const Icon(Icons.edit_outlined),
              onPressed: widget.onEdit ?? () => context.push(AppRoutes.brewEditPath(widget.id)),
            ),
          ],
        ],
      ),
      body: LayoutBuilder(
        builder: (context, constraints) {
          // 2 段組の右の面では Ledger と ReferenceTile を 2 列に並べ、内容を 720 px に収める
          // (WideLayout のガイドライン)。
          final bool twoColumn = widget.embedded && constraints.maxWidth >= 640;
          return _body(context, l10n, brew, twoColumn: twoColumn);
        },
      ),
    );
  }

  Widget _body(
    BuildContext context,
    AppLocalizations l10n,
    Brew? brew, {
    bool twoColumn = false,
  }) {
    if (brew == null) {
      if (_loading) {
        return Center(
          child: Text(
            l10n.loading,
            style: AppTextStyle.body(color: BrewbookTheme.of(context).palette.inkMuted),
          ),
        );
      }
      return Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(AppSpacing.x6),
          child: ErrorBanner(
            message: _errorMessage ?? l10n.errorUnexpected,
            onRetry: _load,
          ),
        ),
      );
    }
    final purchase = brew.purchase;
    final shop = purchase.shop;
    final notes = brew.notes;
    final Widget ledger = Ledger(
      rows: <Widget>[
        LedgerRow(label: l10n.doseLabel, value: _number(brew.doseGrams), unit: l10n.gramUnit),
        LedgerRow(label: l10n.waterLabel, value: _number(brew.waterGrams), unit: l10n.gramUnit),
        LedgerRow(
          label: l10n.waterTempLabel,
          value: _number(brew.waterTempC),
          unit: l10n.celsiusUnit,
        ),
        LedgerRow(
          label: l10n.brewTimeLabel,
          value: brew.brewTimeSeconds?.toString(),
          unit: l10n.secondUnit,
        ),
        LedgerRow(label: l10n.methodLabel, value: brew.method, mono: false),
        LedgerRow(label: l10n.grindSettingLabel, value: brew.grindSetting, mono: false),
        LedgerRow(
          label: l10n.ratingLabel,
          value: brew.rating == null ? null : l10n.ratingValue(brew.rating!.toString()),
        ),
      ],
    );
    final Widget chain = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        SectionLabel(text: l10n.usedBeansLabel),
        const SizedBox(height: AppSpacing.x2),
        ReferenceChain(
          tiles: <Widget>[
            ReferenceTile(
              kind: l10n.purchaseLabel,
              name: _purchaseTileName(context, l10n, purchase),
              onTap: () => context.push(AppRoutes.purchasePath(purchase.id)),
            ),
            ReferenceTile(
              kind: l10n.productLabel,
              name: purchase.product.name,
              onTap: () => context.push(AppRoutes.productEditPath(purchase.product.id)),
            ),
            if (shop != null)
              ReferenceTile(
                kind: l10n.shopLabel,
                name: shop.name,
                onTap: () => context.push(AppRoutes.shopEditPath(shop.id)),
              ),
          ],
        ),
      ],
    );
    final Widget notesBlock = notes == null || notes.isEmpty
        ? const SizedBox.shrink()
        : Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              SectionLabel(text: l10n.notesLabel),
              const SizedBox(height: AppSpacing.x1),
              Text(
                notes,
                style: AppTextStyle.body(color: BrewbookTheme.of(context).palette.ink),
              ),
            ],
          );
    final Widget content = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        _header(context, l10n, brew),
        const SizedBox(height: AppSpacing.x6),
        if (twoColumn)
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Expanded(child: ledger),
              const SizedBox(width: AppSpacing.x8),
              Expanded(child: chain),
            ],
          )
        else ...<Widget>[
          ledger,
          if (notes != null && notes.isNotEmpty) ...<Widget>[
            const SizedBox(height: AppSpacing.x6),
            notesBlock,
          ],
          const SizedBox(height: AppSpacing.x6),
          chain,
        ],
        if (twoColumn && notes != null && notes.isNotEmpty) ...<Widget>[
          const SizedBox(height: AppSpacing.x6),
          notesBlock,
        ],
        if (_errorMessage != null) ...<Widget>[
          const SizedBox(height: AppSpacing.x5),
          ErrorBanner(message: _errorMessage!),
        ],
      ],
    );
    return SingleChildScrollView(
      padding: EdgeInsets.fromLTRB(
        twoColumn ? AppSpacing.x6 : AppSpacing.x4,
        twoColumn ? AppSpacing.x6 : AppSpacing.x4,
        twoColumn ? AppSpacing.x6 : AppSpacing.x4,
        AppSpacing.x8,
      ),
      child: Align(
        alignment: Alignment.topLeft,
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 720),
          child: content,
        ),
      ),
    );
  }

  /// 題 (商品名)、抽出日時、評価。
  Widget _header(BuildContext context, AppLocalizations l10n, Brew brew) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final locale = Localizations.localeOf(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Row(
          children: <Widget>[
            Expanded(
              child: Text(
                brew.purchase.product.name,
                style: AppTextStyle.title(color: brewbook.palette.ink),
              ),
            ),
            if (brew.isArchived) const ArchivedBadge(),
          ],
        ),
        const SizedBox(height: 2),
        Text(
          displayTimestamp(brew.brewedAt, locale),
          style: AppTextStyle.value(color: brewbook.palette.inkMuted),
        ),
        if (brew.rating != null) ...<Widget>[
          const SizedBox(height: AppSpacing.x2),
          Rating(value: brew.rating, showValue: true),
        ],
      ],
    );
  }

  /// 購入のタイルの名前 (購入日 / 重量 / 価格)。商品名は上に出ているので繰り返さない。
  String _purchaseTileName(BuildContext context, AppLocalizations l10n, Purchase purchase) {
    final locale = Localizations.localeOf(context);
    final parts = <String>[displayDay(purchase.purchasedOn, locale)];
    final weight = purchase.weightGrams;
    if (weight != null) {
      parts.add(l10n.gramsValue(formatNumber(weight.toDouble())));
    }
    final amount = purchase.priceAmount;
    if (amount != null) {
      parts.add(l10n.priceValue(amount.toString(), purchase.priceCurrency ?? ''));
    }
    return parts.join(' / ');
  }

  /// 小数の値を表示用にする。無いときは null (Ledger が「未設定」を出す)。
  String? _number(double? value) => value == null ? null : formatNumber(value);
}
