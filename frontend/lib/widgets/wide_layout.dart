import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../l10n/app_localizations.dart';
import '../router/app_router.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';
import 'brewbook_mark.dart';

/// 2 段組にする画面の幅 (px)。これ未満は 1 列 (docs/design/components/WideLayout)。
const double wideLayoutBreakpoint = 840;

/// 一覧の面の幅 (px)。
const double wideListPaneWidth = 400;

/// 幅 840 px 以上の配置。ナビゲーションレールと、一覧 (400 px) と詳細の 2 段組。
///
/// レールの項目は抽出、購入、商品、店、統計、設定。一覧は幅 400 px で右端に line の
/// 罫線を引く。幅 840 px 未満では [WideLayout] を組まず、一覧の行を押すと詳細を上に積む。
class WideLayout extends StatelessWidget {
  const WideLayout({
    super.key,
    required this.railIndex,
    required this.list,
    this.detail,
  });

  /// 選択中のレールの項目 (0: 抽出、1: 購入、2: 商品、3: 店、4: 統計、5: 設定)。
  final int railIndex;

  /// 左の一覧の面 (幅 400 px)。
  final Widget list;

  /// 右の詳細の面。無いときは空にする。
  final Widget? detail;

  @override
  Widget build(BuildContext context) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    return Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        BrewbookNavigationRail(selectedIndex: railIndex),
        SizedBox(
          width: wideListPaneWidth,
          child: DecoratedBox(
            decoration: BoxDecoration(
              border: Border(right: BorderSide(color: brewbook.line)),
            ),
            child: list,
          ),
        ),
        Expanded(child: detail ?? const SizedBox.shrink()),
      ],
    );
  }
}

/// ナビゲーションレール (88 px)。
///
/// 上に印、その下に 6 つの項目。選択中は roast-soft の地に ink、他は ink-muted。
/// 押すとその節へ移る (ホームの上に積まず、入れ替える)。
class BrewbookNavigationRail extends StatelessWidget {
  const BrewbookNavigationRail({super.key, required this.selectedIndex});

  /// 選択中の項目の番号。
  final int selectedIndex;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final List<_RailDestination> destinations = <_RailDestination>[
      _RailDestination(
        label: l10n.brewsLabel,
        icon: Icons.format_list_bulleted_outlined,
        path: AppRoutes.home,
      ),
      _RailDestination(
        label: l10n.purchasesTitle,
        icon: Icons.shopping_bag_outlined,
        path: AppRoutes.purchases,
      ),
      _RailDestination(
        label: l10n.productsTitle,
        icon: Icons.spa_outlined,
        path: AppRoutes.products,
      ),
      _RailDestination(
        label: l10n.shopsTitle,
        icon: Icons.storefront_outlined,
        path: AppRoutes.shops,
      ),
      _RailDestination(
        label: l10n.statsTitle,
        icon: Icons.bar_chart_outlined,
        path: AppRoutes.stats,
      ),
      _RailDestination(
        label: l10n.settingsTitle,
        icon: Icons.settings_outlined,
        path: AppRoutes.settings,
      ),
    ];
    return Material(
      color: brewbook.palette.paper,
      child: Container(
        width: 88,
        padding: const EdgeInsets.symmetric(vertical: AppSpacing.x3),
        decoration: BoxDecoration(
          border: Border(right: BorderSide(color: brewbook.line)),
        ),
        child: Column(
          children: <Widget>[
            const BrewbookMark(width: 40),
            const SizedBox(height: AppSpacing.x4),
            for (int index = 0; index < destinations.length; index++)
              _RailItem(
                destination: destinations[index],
                selected: index == selectedIndex,
              ),
          ],
        ),
      ),
    );
  }
}

/// レールの 1 項目。
class _RailItem extends StatelessWidget {
  const _RailItem({required this.destination, required this.selected});

  final _RailDestination destination;
  final bool selected;

  @override
  Widget build(BuildContext context) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final Color color = selected ? brewbook.palette.ink : brewbook.palette.inkMuted;
    return Padding(
      padding: const EdgeInsets.only(bottom: AppSpacing.x1),
      child: InkWell(
        onTap: () => context.go(destination.path),
        borderRadius: AppRadius.mdAll,
        child: Container(
          width: 72,
          padding: const EdgeInsets.symmetric(vertical: AppSpacing.x2),
          decoration: BoxDecoration(
            color: selected ? brewbook.roastSoft : Colors.transparent,
            borderRadius: AppRadius.mdAll,
          ),
          child: Column(
            children: <Widget>[
              Icon(destination.icon, size: 24, color: color),
              const SizedBox(height: AppSpacing.x1),
              Text(
                destination.label,
                style: AppTextStyle.caption(color: color).copyWith(
                  fontWeight: FontWeight.w500,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// レールの項目の定義。
class _RailDestination {
  const _RailDestination({required this.label, required this.icon, required this.path});

  final String label;
  final IconData icon;
  final String path;
}
