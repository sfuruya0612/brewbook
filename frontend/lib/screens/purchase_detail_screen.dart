import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../api/models.dart';
import '../l10n/app_localizations.dart';
import '../records/record_services.dart';
import '../records/values.dart';
import '../router/app_router.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';
import '../widgets/error_banner.dart';
import '../widgets/error_message.dart';
import '../widgets/ledger.dart';
import '../widgets/list_row.dart';
import '../widgets/reference_tile.dart';
import '../widgets/stats_charts.dart';

/// 購入の詳細の画面 (FR-9、FR-10、UC-6、FR-18)。
///
/// 商品と店をたどれるようにし、写真の差し替えと削除、評価の推移の折れ線グラフを表示する。
/// 幅 840 px 以上の 2 段組では [embedded] を true にし、右の面に出す。
class PurchaseDetailScreen extends StatefulWidget {
  const PurchaseDetailScreen({
    super.key,
    required this.services,
    required this.id,
    this.embedded = false,
    this.onEdit,
  });

  /// 記録の画面が使う依存 (ADR-0007)。
  final RecordServices services;

  /// 表示する購入の ID。
  final String id;

  /// 2 段組の右の面に出すか。戻るの代わりに閉じる操作を置かない。
  final bool embedded;

  /// 編集を開く動き。無いときは編集の経路を上に積む。
  final VoidCallback? onEdit;

  @override
  State<PurchaseDetailScreen> createState() => _PurchaseDetailScreenState();
}

class _PurchaseDetailScreenState extends State<PurchaseDetailScreen> {
  Purchase? _purchase;
  List<RatingHistoryEntry> _ratings = const <RatingHistoryEntry>[];
  String? _errorMessage;
  String? _ratingErrorMessage;
  String? _photoErrorMessage;

  /// 写真のアップロード中か。行の文字を「アップロード中」にして無効にする。
  bool _photoBusy = false;

  /// 読み込みの世代。古い読み込みの応答を捨てるために使う。
  int _loadGeneration = 0;
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

  /// 購入と評価の推移を読み込む。アーカイブ済みでも返る (FR-12)。
  ///
  /// 読み込みが重なると古い応答が新しい表示を上書きするため、読み込みの世代を持ち、
  /// 最新の世代の応答だけを反映する。
  Future<void> _load() async {
    final generation = ++_loadGeneration;
    setState(() {
      _loading = true;
      _errorMessage = null;
      _ratingErrorMessage = null;
    });
    try {
      final purchase = await widget.services.records.purchase(widget.id);
      if (!mounted || generation != _loadGeneration) {
        return;
      }
      setState(() => _purchase = purchase);
      await _loadRatings(generation);
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

  /// 評価の推移を読み込む (FR-18)。
  ///
  /// 読み込みの失敗は、購入の表示を残したままグラフの区画にだけ出す。
  Future<void> _loadRatings(int generation) async {
    try {
      final ratings = await widget.services.stats.ratingHistory(widget.id);
      if (!mounted || generation != _loadGeneration) {
        return;
      }
      setState(() => _ratings = ratings);
    } catch (error) {
      if (!mounted || generation != _loadGeneration) {
        return;
      }
      setState(() => _ratingErrorMessage = messageForError(error, AppLocalizations.of(context)));
    }
  }

  /// アーカイブとアーカイブ解除を行う (FR-12)。
  Future<void> _toggleArchived() async {
    final purchase = _purchase;
    if (purchase == null) {
      return;
    }
    final l10n = AppLocalizations.of(context);
    try {
      // 応答の行で画面の状態を更新する (アーカイブの切り替えを画面に反映する)。
      final updated = await widget.services.records.setPurchaseArchived(purchase.id, !purchase.isArchived);
      widget.services.markRecordsChanged();
      if (!mounted) {
        return;
      }
      setState(() => _purchase = updated);
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

  /// 写真を選び直してアップロードする (FR-10)。
  Future<void> _replacePhoto() async {
    final l10n = AppLocalizations.of(context);
    setState(() {
      _photoBusy = true;
      _photoErrorMessage = null;
    });
    try {
      final photo = await widget.services.picker.pickPhoto();
      if (photo == null) {
        return;
      }
      final converted = await widget.services.converter.convertJpeg(photo.bytes);
      await widget.services.uploader.upload(purchaseId: widget.id, image: converted);
      widget.services.markRecordsChanged();
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() => _photoErrorMessage = messageForError(error, l10n));
    } finally {
      if (mounted) {
        setState(() => _photoBusy = false);
      }
    }
  }

  /// 写真を削除する (FR-10)。
  Future<void> _deletePhoto() async {
    final l10n = AppLocalizations.of(context);
    setState(() {
      _photoBusy = true;
      _photoErrorMessage = null;
    });
    try {
      await widget.services.records.deletePhoto(widget.id);
      widget.services.markRecordsChanged();
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() => _photoErrorMessage = messageForError(error, l10n));
    } finally {
      if (mounted) {
        setState(() => _photoBusy = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final purchase = _purchase;
    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.purchaseDetailTitle),
        automaticallyImplyLeading: !widget.embedded,
        actions: <Widget>[
          if (purchase != null) ...<Widget>[
            IconButton(
              tooltip: purchase.isArchived ? l10n.unarchiveButton : l10n.archiveButton,
              icon: Icon(
                purchase.isArchived ? Icons.unarchive_outlined : Icons.archive_outlined,
              ),
              onPressed: _toggleArchived,
            ),
            IconButton(
              tooltip: l10n.editButton,
              icon: const Icon(Icons.edit_outlined),
              onPressed:
                  widget.onEdit ?? () => context.push(AppRoutes.purchaseEditPath(widget.id)),
            ),
          ],
        ],
      ),
      body: LayoutBuilder(
        builder: (context, constraints) {
          // 2 段組の右の面では Ledger と参照先とグラフを 2 列に並べ、内容を 720 px に収める
          // (WideLayout のガイドライン)。
          final bool twoColumn = widget.embedded && constraints.maxWidth >= 640;
          return _body(context, l10n, purchase, twoColumn: twoColumn);
        },
      ),
    );
  }

  Widget _body(
    BuildContext context,
    AppLocalizations l10n,
    Purchase? purchase, {
    bool twoColumn = false,
  }) {
    if (purchase == null) {
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
    final locale = Localizations.localeOf(context);
    final Widget ledger = Ledger(
      rows: <Widget>[
        LedgerRow(
          label: l10n.purchasedOnLabel,
          value: displayDay(purchase.purchasedOn, locale),
        ),
        LedgerRow(label: l10n.roast, value: valueOrUnset(purchase.roast, l10n), mono: false),
        LedgerRow(
          label: l10n.roastDate,
          value: purchase.roastDate == null ? null : displayDay(purchase.roastDate!, locale),
        ),
        LedgerRow(
          label: l10n.priceLabel,
          value: purchase.priceAmount?.toString(),
          unit: purchase.priceCurrency,
        ),
        LedgerRow(
          label: l10n.weightLabel,
          value: purchase.weightGrams?.toString(),
          unit: l10n.gramUnit,
        ),
        LedgerRow(label: l10n.photoLabel, trailing: _photoActions(l10n)),
      ],
    );
    final Widget chart = _ratingHistory(context, l10n);
    final Widget chain = ReferenceChain(
      tiles: <Widget>[
        ReferenceTile(
          kind: l10n.productLabel,
          name: purchase.product.name,
          onTap: () => context.push(AppRoutes.productEditPath(purchase.product.id)),
        ),
        if (purchase.shop != null)
          ReferenceTile(
            kind: l10n.shopLabel,
            name: purchase.shop!.name,
            onTap: () => context.push(AppRoutes.shopEditPath(purchase.shop!.id)),
          ),
      ],
    );
    final Widget content = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        _header(context, l10n, purchase),
        const SizedBox(height: AppSpacing.x6),
        if (twoColumn)
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Expanded(child: ledger),
              const SizedBox(width: AppSpacing.x8),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: <Widget>[
                    chart,
                    const SizedBox(height: AppSpacing.x6),
                    chain,
                  ],
                ),
              ),
            ],
          )
        else ...<Widget>[
          ledger,
          const SizedBox(height: AppSpacing.x6),
          chart,
          const SizedBox(height: AppSpacing.x6),
          chain,
        ],
        if (_photoErrorMessage != null) ...<Widget>[
          const SizedBox(height: AppSpacing.x5),
          ErrorBanner(message: _photoErrorMessage!, onRetry: _replacePhoto),
        ],
        if (_ratingErrorMessage != null) ...<Widget>[
          const SizedBox(height: AppSpacing.x5),
          ErrorBanner(message: _ratingErrorMessage!, onRetry: _load),
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

  /// 題 (写真、商品名、店、タグ)。
  Widget _header(BuildContext context, AppLocalizations l10n, Purchase purchase) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        ClipRRect(
          borderRadius: AppRadius.smAll,
          child: Container(
            width: 112,
            height: 112,
            color: brewbook.palette.paperSunken,
            alignment: Alignment.center,
            child: purchase.photoKey == null
                ? Icon(Icons.photo_camera_outlined, size: 32, color: brewbook.inkFaint)
                : Image.network(
                    widget.services.records.photoUrl(purchase.id).toString(),
                    key: const Key('purchase-photo'),
                    width: 112,
                    height: 112,
                    fit: BoxFit.cover,
                    // 写真が取得できないとき (古い記録、一時的な失敗) も画面は壊さない。
                    errorBuilder: (context, error, stackTrace) =>
                        Icon(Icons.photo_camera_outlined, size: 32, color: brewbook.inkFaint),
                  ),
          ),
        ),
        const SizedBox(width: AppSpacing.x4),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(
                purchase.product.name,
                style: AppTextStyle.title(color: brewbook.palette.ink),
              ),
              if (purchase.shop != null) ...<Widget>[
                const SizedBox(height: 2),
                Text(
                  purchase.shop!.name,
                  style: AppTextStyle.caption(color: brewbook.palette.inkMuted),
                ),
              ],
              if (purchase.product.flavorNotes.isNotEmpty) ...<Widget>[
                const SizedBox(height: AppSpacing.x2),
                Wrap(
                  spacing: AppSpacing.x2,
                  runSpacing: AppSpacing.x1,
                  children: <Widget>[
                    for (final note in purchase.product.flavorNotes) TagChip(label: note),
                  ],
                ),
              ],
              if (purchase.isArchived) ...<Widget>[
                const SizedBox(height: AppSpacing.x2),
                const ArchivedBadge(),
              ],
            ],
          ),
        ),
      ],
    );
  }

  /// 写真の行の操作 (差し替えと削除。FR-10)。
  Widget _photoActions(AppLocalizations l10n) {
    if (_photoBusy) {
      return Text(
        l10n.uploadingLabel,
        style: AppTextStyle.body(color: BrewbookTheme.of(context).palette.inkFaint),
      );
    }
    final bool hasPhoto = _purchase?.photoKey != null;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        TextButton(
          onPressed: _replacePhoto,
          child: Text(hasPhoto ? l10n.photoReplaceButton : l10n.photoSelectButton),
        ),
        if (hasPhoto)
          TextButton(
            onPressed: _deletePhoto,
            child: Text(l10n.photoDeleteButton),
          ),
      ],
    );
  }

  /// 評価の推移の折れ線グラフ (FR-18)。
  Widget _ratingHistory(BuildContext context, AppLocalizations l10n) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Row(
          children: <Widget>[
            Expanded(
              child: Text(
                l10n.ratingHistoryTitle,
                style: AppTextStyle.heading(color: brewbook.palette.ink),
              ),
            ),
            if (_ratings.isNotEmpty)
              Builder(
                builder: (context) {
                  final String countLabel = '${_ratings.length} ${l10n.statsUnitCups}';
                  return Text(
                    countLabel,
                    style: AppTextStyle.caption(color: brewbook.palette.inkMuted),
                  );
                },
              ),
          ],
        ),
        const SizedBox(height: AppSpacing.x2),
        RatingHistoryChart(entries: _ratings),
      ],
    );
  }
}
