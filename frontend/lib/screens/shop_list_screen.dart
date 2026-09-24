import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../api/models.dart';
import '../l10n/app_localizations.dart';
import '../records/record_services.dart';
import '../router/app_router.dart';
import '../widgets/record_list_view.dart';

/// 店の一覧の画面 (FR-6、FR-12)。
///
/// 行から店の編集を開き、行の末尾からアーカイブとアーカイブ解除ができる。
class ShopListScreen extends StatelessWidget {
  const ShopListScreen({super.key, required this.services});

  /// 記録の画面が使う依存 (ADR-0007)。
  final RecordServices services;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(title: Text(l10n.shopsTitle)),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: () => context.push(AppRoutes.shopNew),
        icon: const Icon(Icons.add),
        label: Text(l10n.newShopButton),
      ),
      body: RecordListView<Shop>(
        load: ({cursor, required includeArchived}) => services.records.shops(
          cursor: cursor,
          includeArchived: includeArchived,
        ),
        services: services,
        isArchived: (shop) => shop.isArchived,
        setArchived: (shop, archived) => services.records.setShopArchived(shop.id, archived),
        title: (context, shop) => Text(shop.name),
        subtitle: (context, shop) {
          final address = shop.address;
          return address == null || address.isEmpty ? null : Text(address);
        },
        onTap: (context, shop) => context.push(AppRoutes.shopEditPath(shop.id)),
      ),
    );
  }
}
