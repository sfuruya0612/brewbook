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
import '../widgets/stats_charts.dart';

/// 購入の詳細の画面 (FR-9、FR-10、UC-6、FR-18)。
///
/// 商品と店をたどれるようにし、写真と評価の推移の折れ線グラフを表示する。
class PurchaseDetailScreen extends StatefulWidget {
  const PurchaseDetailScreen({super.key, required this.services, required this.id});

  /// 記録の画面が使う依存 (ADR-0007)。
  final RecordServices services;

  /// 表示する購入の ID。
  final String id;

  @override
  State<PurchaseDetailScreen> createState() => _PurchaseDetailScreenState();
}

class _PurchaseDetailScreenState extends State<PurchaseDetailScreen> {
  Purchase? _purchase;
  List<RatingHistoryEntry> _ratings = const <RatingHistoryEntry>[];
  String? _errorMessage;
  String? _ratingErrorMessage;

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

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final purchase = _purchase;
    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.purchaseDetailTitle),
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
              onPressed: () => context.push(AppRoutes.purchaseEditPath(widget.id)),
            ),
          ],
        ],
      ),
      body: _body(context, l10n, purchase),
    );
  }

  Widget _body(BuildContext context, AppLocalizations l10n, Purchase? purchase) {
    if (purchase == null) {
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
    return SingleChildScrollView(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          DetailRow(
            label: l10n.productLabel,
            value: purchase.product.name,
            onTap: () => context.push(AppRoutes.productEditPath(purchase.product.id)),
          ),
          DetailRow(
            label: l10n.shopLabel,
            value: purchase.shop?.name ?? l10n.unsetLabel,
            onTap: purchase.shop == null
                ? null
                : () => context.push(AppRoutes.shopEditPath(purchase.shop!.id)),
          ),
          DetailRow(
            label: l10n.purchasedOnLabel,
            value: displayDay(purchase.purchasedOn, Localizations.localeOf(context)),
          ),
          DetailRow(label: l10n.roast, value: valueOrUnset(purchase.roast, l10n)),
          DetailRow(
            label: l10n.roastDate,
            value: purchase.roastDate == null
                ? l10n.unsetLabel
                : displayDay(purchase.roastDate!, Localizations.localeOf(context)),
          ),
          DetailRow(label: l10n.priceLabel, value: _price(l10n, purchase)),
          DetailRow(
            label: l10n.weightLabel,
            value: purchase.weightGrams == null
                ? l10n.unsetLabel
                : l10n.gramsValue(formatNumber(purchase.weightGrams!.toDouble())),
          ),
          Padding(
            padding: const EdgeInsets.all(16),
            child: Text(l10n.photoLabel, style: Theme.of(context).textTheme.titleMedium),
          ),
          _photo(context, l10n, purchase),
          Padding(
            padding: const EdgeInsets.all(16),
            child: Text(
              l10n.ratingHistoryTitle,
              style: Theme.of(context).textTheme.titleMedium,
            ),
          ),
          // その購入の評価の推移の折れ線グラフ (FR-18)。
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16),
            child: RatingHistoryChart(entries: _ratings),
          ),
          if (_ratingErrorMessage != null) ...<Widget>[
            const SizedBox(height: 16),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              child: ErrorBanner(message: _ratingErrorMessage!, onRetry: _load),
            ),
          ],
          if (_errorMessage != null) ...<Widget>[
            const SizedBox(height: 16),
            ErrorBanner(message: _errorMessage!),
          ],
        ],
      ),
    );
  }

  /// 写真を表示する (FR-10)。写真が無い購入ではその旨を出す。
  Widget _photo(BuildContext context, AppLocalizations l10n, Purchase purchase) {
    if (purchase.photoKey == null) {
      return Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16),
        child: Text(l10n.photoNoneLabel),
      );
    }
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16),
      child: Image.network(
        widget.services.records.photoUrl(purchase.id).toString(),
        key: const Key('purchase-photo'),
        // 写真が取得できないとき (古い記録、一時的な失敗) も画面は壊さない。
        errorBuilder: (context, error, stackTrace) => Text(l10n.photoNoneLabel),
      ),
    );
  }

  /// 価格を通貨コードとともに表示する (FR-9)。
  String _price(AppLocalizations l10n, Purchase purchase) {
    final amount = purchase.priceAmount;
    if (amount == null) {
      return l10n.unsetLabel;
    }
    return l10n.priceValue(amount.toString(), purchase.priceCurrency ?? '');
  }
}
