import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../api/models.dart';
import '../l10n/app_localizations.dart';
import '../records/record_services.dart';
import '../router/app_router.dart';
import '../widgets/record_list_view.dart';

/// 商品の一覧の画面 (FR-7、FR-8、FR-12)。
///
/// 行から商品の編集を開き、行の末尾からアーカイブとアーカイブ解除ができる。
class ProductListScreen extends StatelessWidget {
  const ProductListScreen({super.key, required this.services});

  /// 記録の画面が使う依存 (ADR-0007)。
  final RecordServices services;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Scaffold(
      appBar: AppBar(title: Text(l10n.productsTitle)),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: () => context.push(AppRoutes.productNew),
        icon: const Icon(Icons.add),
        label: Text(l10n.newProductButton),
      ),
      body: RecordListView<Product>(
        load: ({cursor, required includeArchived}) => services.records.products(
          cursor: cursor,
          includeArchived: includeArchived,
        ),
        services: services,
        isArchived: (product) => product.isArchived,
        setArchived: (product, archived) =>
            services.records.setProductArchived(product.id, archived),
        title: (context, product) => Text(product.name),
        subtitle: (context, product) {
          final producer = product.producer;
          return producer == null || producer.isEmpty ? null : Text(producer);
        },
        onTap: (context, product) => context.push(AppRoutes.productEditPath(product.id)),
      ),
    );
  }
}
