import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../api/models.dart';
import '../l10n/app_localizations.dart';
import '../records/record_services.dart';
import '../records/values.dart';
import '../router/app_router.dart';
import '../theme/app_theme.dart';
import '../widgets/list_row.dart';
import '../widgets/record_list_view.dart';
import '../widgets/wide_layout.dart';
import 'purchase_detail_screen.dart';
import 'purchase_form_screen.dart';

/// 購入の一覧の画面 (FR-9、FR-12)。
///
/// 行から購入の詳細を開く。幅 840 px 以上では一覧と詳細を 2 段組にし、行を押すと
/// 右の面に詳細やフォームを出す。
class PurchaseListScreen extends StatefulWidget {
  const PurchaseListScreen({super.key, required this.services});

  /// 記録の画面が使う依存 (ADR-0007)。
  final RecordServices services;

  @override
  State<PurchaseListScreen> createState() => _PurchaseListScreenState();
}

class _PurchaseListScreenState extends State<PurchaseListScreen> {
  /// 広い画面で選択中の購入の ID。
  String? _selectedId;

  /// 広い画面の右の面で編集中か。
  bool _editing = false;

  /// 広い画面の右の面で新規の登録中か。
  bool _creating = false;

  /// 一覧の行の補足 (購入日と店。FR-16)。
  Widget _subtitle(BuildContext context, Purchase purchase) {
    final l10n = AppLocalizations.of(context);
    final date = displayDay(purchase.purchasedOn, Localizations.localeOf(context));
    final shop = purchase.shop;
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final String? shopSuffix = shop == null ? null : l10n.rowSubtitleSeparator + shop.name;
    return Text.rich(
      TextSpan(
        children: <InlineSpan>[
          TextSpan(
            text: date,
            style: AppTextStyle.mono(size: 12, lineHeight: 16, color: brewbook.palette.inkMuted),
          ),
          if (shopSuffix != null) TextSpan(text: shopSuffix),
        ],
      ),
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
    );
  }

  /// 一覧の行の右端 (重量と価格)。
  Widget _trailing(BuildContext context, AppLocalizations l10n, Purchase purchase) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final int? weight = purchase.weightGrams;
    final int? amount = purchase.priceAmount;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.end,
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Text(
          weight == null ? l10n.unsetLabel : l10n.gramsValue(formatNumber(weight.toDouble())),
          style: AppTextStyle.value(
            color: weight == null ? brewbook.inkFaint : brewbook.palette.ink,
          ),
        ),
        Text(
          amount == null ? l10n.unsetLabel : l10n.priceValue(amount.toString(), purchase.priceCurrency ?? ''),
          style: AppTextStyle.caption(color: brewbook.palette.inkMuted),
        ),
      ],
    );
  }

  /// 一覧の面。幅 840 px 未満では画面全体になる。
  Widget _listPane(BuildContext context, AppLocalizations l10n, bool wide) {
    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.purchasesTitle),
        automaticallyImplyLeading: !wide,
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: () {
          if (wide) {
            setState(() {
              _creating = true;
              _editing = false;
            });
          } else {
            context.push(AppRoutes.purchaseNew);
          }
        },
        icon: const Icon(Icons.add_outlined),
        label: Text(l10n.newPurchaseButton),
      ),
      body: RecordListView<Purchase>(
        load: ({cursor, required includeArchived}) => widget.services.records.purchases(
          cursor: cursor,
          includeArchived: includeArchived,
        ),
        services: widget.services,
        emptyHint: l10n.purchasesEmptyHint,
        isSelected: (purchase) => wide && !_creating && purchase.id == _selectedId,
        onTap: (context, purchase) {
          if (wide) {
            setState(() {
              _selectedId = purchase.id;
              _editing = false;
              _creating = false;
            });
          } else {
            context.push(AppRoutes.purchasePath(purchase.id));
          }
        },
        row: (context, purchase, {required selected, required onTap}) => ListRow(
          leading: ListThumb(
            image: purchase.photoKey == null
                ? null
                : NetworkImage(widget.services.records.photoUrl(purchase.id).toString()),
          ),
          title: purchase.product.name,
          subtitle: _subtitle(context, purchase),
          trailing: _trailing(context, l10n, purchase),
          archived: purchase.isArchived,
          selected: selected,
          onTap: onTap,
        ),
      ),
    );
  }

  /// 右の面 (詳細、編集、新規の登録)。
  Widget? _detailPane(BuildContext context) {
    if (_creating) {
      return PurchaseFormScreen(
        services: widget.services,
        embedded: true,
        onClose: () => setState(() => _creating = false),
        onSaved: () => setState(() => _creating = false),
      );
    }
    final selectedId = _selectedId;
    if (selectedId == null) {
      return null;
    }
    if (_editing) {
      return PurchaseFormScreen(
        services: widget.services,
        id: selectedId,
        embedded: true,
        onClose: () => setState(() => _editing = false),
        onSaved: () => setState(() => _editing = false),
      );
    }
    return PurchaseDetailScreen(
      services: widget.services,
      id: selectedId,
      embedded: true,
      onEdit: () => setState(() => _editing = true),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return LayoutBuilder(
      builder: (context, constraints) {
        final wide = constraints.maxWidth >= wideLayoutBreakpoint;
        final list = _listPane(context, l10n, wide);
        if (!wide) {
          return list;
        }
        return WideLayout(railIndex: 1, list: list, detail: _detailPane(context));
      },
    );
  }
}
