import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../api/models.dart';
import '../l10n/app_localizations.dart';
import '../records/record_services.dart';
import '../router/app_router.dart';
import '../theme/app_theme.dart';
import '../widgets/list_row.dart';
import '../widgets/record_list_view.dart';
import '../widgets/wide_layout.dart';
import 'product_form_screen.dart';

/// 商品の一覧の画面 (FR-7、FR-8、FR-12)。
///
/// 行から商品の編集を開く。幅 840 px 以上では一覧とフォームを 2 段組にし、行を押すと
/// 右の面にフォームを出す。
class ProductListScreen extends StatefulWidget {
  const ProductListScreen({super.key, required this.services});

  /// 記録の画面が使う依存 (ADR-0007)。
  final RecordServices services;

  @override
  State<ProductListScreen> createState() => _ProductListScreenState();
}

class _ProductListScreenState extends State<ProductListScreen> {
  /// 広い画面で選択中の商品の ID。
  String? _selectedId;

  /// 広い画面の右の面で新規の登録中か。
  bool _creating = false;

  /// 一覧の行の補足 (生産地 / 精製方法 / 品種)。
  String? _subtitle(Product product) {
    final String location = <String?>[product.region, product.origin]
        .whereType<String>()
        .where((value) => value.isNotEmpty)
        .join(', ');
    final List<String> parts = <String>[
      if (location.isNotEmpty) location,
      if (product.process != null && product.process!.isNotEmpty) product.process!,
      if (product.variety != null && product.variety!.isNotEmpty) product.variety!,
    ];
    return parts.isEmpty ? null : parts.join(' / ');
  }

  /// 一覧の面。幅 840 px 未満では画面全体になる。
  Widget _listPane(BuildContext context, AppLocalizations l10n, bool wide) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.productsTitle),
        automaticallyImplyLeading: !wide,
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: () {
          if (wide) {
            setState(() {
              _creating = true;
              _selectedId = null;
            });
          } else {
            context.push(AppRoutes.productNew);
          }
        },
        icon: const Icon(Icons.add_outlined),
        label: Text(l10n.newProductButton),
      ),
      body: RecordListView<Product>(
        load: ({cursor, required includeArchived}) => widget.services.records.products(
          cursor: cursor,
          includeArchived: includeArchived,
        ),
        services: widget.services,
        emptyHint: l10n.productsEmptyHint,
        isSelected: (product) => wide && !_creating && product.id == _selectedId,
        onTap: (context, product) {
          if (wide) {
            setState(() {
              _selectedId = product.id;
              _creating = false;
            });
          } else {
            context.push(AppRoutes.productEditPath(product.id));
          }
        },
        row: (context, product, {required selected, required onTap}) {
          final subtitle = _subtitle(product);
          return ListRow(
            title: product.name,
            subtitle: subtitle == null ? null : Text(subtitle),
            tags: <Widget>[
              for (final note in product.flavorNotes) TagChip(label: note),
            ],
            trailing: Icon(
              Icons.chevron_right_outlined,
              size: 20,
              color: brewbook.palette.inkMuted,
            ),
            archived: product.isArchived,
            selected: selected,
            onTap: onTap,
          );
        },
      ),
    );
  }

  /// 右の面 (編集、新規の登録)。
  Widget? _detailPane(BuildContext context) {
    if (_creating) {
      return ProductFormScreen(
        services: widget.services,
        embedded: true,
        onClose: () => setState(() => _creating = false),
        // 一覧は保存した商品を使わない (FR-19 の導線だけが結果を受け取る)。
        onSaved: (_) => setState(() => _creating = false),
      );
    }
    final selectedId = _selectedId;
    if (selectedId == null) {
      return null;
    }
    return ProductFormScreen(
      services: widget.services,
      id: selectedId,
      embedded: true,
      onClose: () => setState(() => _selectedId = null),
      onSaved: (_) => setState(() => _selectedId = null),
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
        return WideLayout(railIndex: 2, list: list, detail: _detailPane(context));
      },
    );
  }
}
