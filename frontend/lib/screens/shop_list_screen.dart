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
import 'shop_form_screen.dart';

/// 店の一覧の画面 (FR-6、FR-12)。
///
/// 行から店の編集を開く。幅 840 px 以上では一覧とフォームを 2 段組にし、行を押すと
/// 右の面にフォームを出す。
class ShopListScreen extends StatefulWidget {
  const ShopListScreen({super.key, required this.services});

  /// 記録の画面が使う依存 (ADR-0007)。
  final RecordServices services;

  @override
  State<ShopListScreen> createState() => _ShopListScreenState();
}

class _ShopListScreenState extends State<ShopListScreen> {
  /// 広い画面で選択中の店の ID。
  String? _selectedId;

  /// 広い画面の右の面で新規の登録中か。
  bool _creating = false;

  /// 一覧の面。幅 840 px 未満では画面全体になる。
  Widget _listPane(BuildContext context, AppLocalizations l10n, bool wide) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.shopsTitle),
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
            context.push(AppRoutes.shopNew);
          }
        },
        icon: const Icon(Icons.add_outlined),
        label: Text(l10n.newShopButton),
      ),
      body: RecordListView<Shop>(
        load: ({cursor, required includeArchived}) => widget.services.records.shops(
          cursor: cursor,
          includeArchived: includeArchived,
        ),
        services: widget.services,
        emptyHint: l10n.shopsEmptyHint,
        isSelected: (shop) => wide && !_creating && shop.id == _selectedId,
        onTap: (context, shop) {
          if (wide) {
            setState(() {
              _selectedId = shop.id;
              _creating = false;
            });
          } else {
            context.push(AppRoutes.shopEditPath(shop.id));
          }
        },
        row: (context, shop, {required selected, required onTap}) {
          final address = shop.address;
          return ListRow(
            title: shop.name,
            subtitle: Text(
              address == null || address.isEmpty ? l10n.addressUnset : address,
              style: address == null || address.isEmpty
                  ? AppTextStyle.caption(color: brewbook.palette.inkMuted)
                  : null,
            ),
            trailing: Icon(
              Icons.chevron_right_outlined,
              size: 20,
              color: brewbook.palette.inkMuted,
            ),
            archived: shop.isArchived,
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
      return ShopFormScreen(
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
    return ShopFormScreen(
      services: widget.services,
      id: selectedId,
      embedded: true,
      onClose: () => setState(() => _selectedId = null),
      onSaved: () => setState(() => _selectedId = null),
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
        return WideLayout(railIndex: 3, list: list, detail: _detailPane(context));
      },
    );
  }
}
