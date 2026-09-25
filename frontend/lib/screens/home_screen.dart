import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../api/models.dart';
import '../auth/auth_controller.dart';
import '../auth/auth_scope.dart';
import '../l10n/app_localizations.dart';
import '../records/record_services.dart';
import '../records/values.dart';
import '../router/app_router.dart';
import '../widgets/error_banner.dart';
import '../widgets/error_message.dart';
import '../widgets/record_list_view.dart';

/// ホームの画面 (抽出の一覧。FR-11、FR-12)。
///
/// 一覧から抽出の詳細を開き、末尾までスクロールすると次のページを読む。抽出の登録は
/// この画面から始め、保存完了までの最短経路を短く保つ (PRD の成功指標)。
class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key, required this.services});

  /// 記録の画面が使う依存 (ADR-0007)。
  final RecordServices services;

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  bool _busy = false;

  Future<void> _logout() async {
    final l10n = AppLocalizations.of(context);
    final controller = AuthScope.read(context);
    setState(() => _busy = true);
    try {
      await controller.logout();
    } catch (error) {
      if (!mounted) {
        return;
      }
      // ログアウトの失敗は再試行を促す文言を出す (ADR-0007)。
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(messageForError(error, l10n))),
      );
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _retry() => AuthScope.read(context).check();

  /// 抽出の一覧の行の補足 (日時と店。FR-16)。
  String _brewSubtitle(BuildContext context, Brew brew) {
    final l10n = AppLocalizations.of(context);
    final date = displayTimestamp(brew.brewedAt, Localizations.localeOf(context));
    final shop = brew.purchase.shop;
    if (shop == null) {
      return l10n.brewRowSubtitleNoShop(date);
    }
    return l10n.brewRowSubtitle(date, shop.name);
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final controller = AuthScope.of(context);
    final signedIn = controller.status == SessionStatus.signedIn;
    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.homeTitle),
        actions: <Widget>[
          if (signedIn)
            PopupMenuButton<String>(
              tooltip: l10n.menuTooltip,
              // 一覧はホームの上に積み、ホームへ戻れるようにする (戻るボタンが付く)。
              onSelected: (path) => context.push(path),
              itemBuilder: (context) => <PopupMenuEntry<String>>[
                PopupMenuItem<String>(
                  value: AppRoutes.purchases,
                  child: Text(l10n.purchasesTitle),
                ),
                PopupMenuItem<String>(
                  value: AppRoutes.products,
                  child: Text(l10n.productsTitle),
                ),
                PopupMenuItem<String>(
                  value: AppRoutes.shops,
                  child: Text(l10n.shopsTitle),
                ),
                PopupMenuItem<String>(
                  value: AppRoutes.stats,
                  child: Text(l10n.statsTitle),
                ),
              ],
            ),
          if (signedIn)
            TextButton(
              onPressed: _busy ? null : _logout,
              child: Text(l10n.logoutButton),
            ),
        ],
      ),
      floatingActionButton: signedIn
          ? FloatingActionButton.extended(
              onPressed: () => context.push(AppRoutes.brewNew),
              icon: const Icon(Icons.add),
              label: Text(l10n.newBrewButton),
            )
          : null,
      body: switch (controller.status) {
        SessionStatus.signedIn => RecordListView<Brew>(
          load: ({cursor, required includeArchived}) => widget.services.records.brews(
            cursor: cursor,
            includeArchived: includeArchived,
          ),
          services: widget.services,
          isArchived: (brew) => brew.isArchived,
          setArchived: (brew, archived) =>
              widget.services.records.setBrewArchived(brew.id, archived),
          title: (context, brew) => Text(brew.purchase.product.name),
          subtitle: (context, brew) => Text(_brewSubtitle(context, brew)),
          onTap: (context, brew) => context.push(AppRoutes.brewPath(brew.id)),
        ),
        // 失敗の原因 (ネットワーク、500 など) に応じた文言にする。
        SessionStatus.unknown => Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(24),
            child: ErrorBanner(
              message: controller.unknownError == null
                  ? l10n.errorNetwork
                  : messageForError(controller.unknownError!, l10n),
              onRetry: _retry,
            ),
          ),
        ),
        _ => Center(child: Text(l10n.loading)),
      },
    );
  }
}
