import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../api/models.dart';
import '../l10n/app_localizations.dart';
import '../records/record_services.dart';
import '../records/values.dart';
import '../router/app_router.dart';
import '../widgets/record_list_view.dart';

/// 購入の一覧の画面 (FR-9、FR-12)。
///
/// 行から購入の詳細を開き、行の末尾からアーカイブとアーカイブ解除ができる。
class PurchaseListScreen extends StatelessWidget {
  const PurchaseListScreen({super.key, required this.services});

  /// 記録の画面が使う依存 (ADR-0007)。
  final RecordServices services;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(title: Text(l10n.purchasesTitle)),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: () => context.push(AppRoutes.purchaseNew),
        icon: const Icon(Icons.add),
        label: Text(l10n.newPurchaseButton),
      ),
      body: RecordListView<Purchase>(
        load: ({cursor, required includeArchived}) => services.records.purchases(
          cursor: cursor,
          includeArchived: includeArchived,
        ),
        services: services,
        isArchived: (purchase) => purchase.isArchived,
        setArchived: (purchase, archived) =>
            services.records.setPurchaseArchived(purchase.id, archived),
        title: (context, purchase) => Text(purchase.product.name),
        subtitle: (context, purchase) => Text(
          l10n.purchaseRowSubtitle(
            displayDay(purchase.purchasedOn, Localizations.localeOf(context)),
            purchase.product.name,
          ),
        ),
        onTap: (context, purchase) => context.push(AppRoutes.purchasePath(purchase.id)),
      ),
    );
  }
}
