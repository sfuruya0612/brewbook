import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import '../api/models.dart';
import '../auth/auth_controller.dart';
import '../auth/auth_scope.dart';
import '../l10n/app_localizations.dart';
import '../records/record_services.dart';
import '../records/values.dart';
import '../router/app_router.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';
import '../widgets/error_banner.dart';
import '../widgets/error_message.dart';
import '../widgets/list_row.dart';
import '../widgets/rating.dart';
import '../widgets/record_list_view.dart';
import '../widgets/wide_layout.dart';
import 'brew_detail_screen.dart';
import 'brew_form_screen.dart';

/// ホームの画面 (抽出の一覧。FR-11、FR-12)。
///
/// 一覧から抽出の詳細を開き、末尾までスクロールすると次のページを読む。抽出の登録は
/// この画面から始め、保存完了までの最短経路を短く保つ (PRD の成功指標)。
/// 幅 840 px 以上では一覧と詳細を 2 段組にし、行を押すと右の面に詳細を出す。
class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key, required this.services});

  /// 記録の画面が使う依存 (ADR-0007)。
  final RecordServices services;

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  /// 広い画面で選択中の抽出の ID。
  String? _selectedBrewId;

  /// 広い画面の右の面で編集中か。
  bool _editing = false;

  /// 広い画面の右の面で新規の登録中か。
  bool _creating = false;

  Future<void> _logout() async {
    final l10n = AppLocalizations.of(context);
    final controller = AuthScope.read(context);
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
    }
  }

  Future<void> _retry() => AuthScope.read(context).check();

  /// 抽出の一覧の行の補足 (日時と店。FR-16)。
  Widget _brewSubtitle(BuildContext context, Brew brew) {
    final l10n = AppLocalizations.of(context);
    final date = displayTimestamp(brew.brewedAt, Localizations.localeOf(context));
    final shop = brew.purchase.shop;
    if (shop == null) {
      return RowValueText(text: l10n.brewRowSubtitleNoShop(date));
    }
    final String shopSuffix = l10n.rowSubtitleSeparator + shop.name;
    return Text.rich(
      TextSpan(
        children: <InlineSpan>[
          TextSpan(
            text: date,
            style: AppTextStyle.mono(
              size: 12,
              lineHeight: 16,
              color: BrewbookTheme.of(context).palette.inkMuted,
            ),
          ),
          TextSpan(text: shopSuffix),
        ],
      ),
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
    );
  }

  /// 抽出の一覧の見出しと操作 (ホームの AppBar)。
  PreferredSizeWidget _appBar(BuildContext context, AppLocalizations l10n, bool wide) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final controller = AuthScope.of(context);
    final signedIn = controller.status == SessionStatus.signedIn;
    return AppBar(
      // ホームだけ題を wordmark の「brewbook」にする (AppBar のガイドライン)。
      title: Text(l10n.homeTitle, style: AppTextStyle.appBarWordmark(color: brewbook.palette.ink)),
      actions: <Widget>[
        // 広い画面ではレールが他の節への入口を持つため、メニューは置かない。
        if (signedIn && !wide)
          PopupMenuButton<String>(
            tooltip: l10n.menuTooltip,
            // 一覧はホームの上に積み、ホームへ戻れるようにする (戻るボタンが付く)。
            onSelected: (path) {
              if (path == _logoutMenuValue) {
                _logout();
              } else {
                context.push(path);
              }
            },
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
              PopupMenuItem<String>(
                value: AppRoutes.settings,
                child: Text(l10n.settingsTitle),
              ),
              const PopupMenuDivider(),
              PopupMenuItem<String>(
                value: _logoutMenuValue,
                child: Text(l10n.logoutButton),
              ),
            ],
          ),
      ],
    );
  }

  /// 一覧の面。幅 840 px 未満では画面全体になる。
  Widget _listPane(BuildContext context, AppLocalizations l10n, bool wide) {
    final controller = AuthScope.of(context);
    final signedIn = controller.status == SessionStatus.signedIn;
    return Scaffold(
      appBar: _appBar(context, l10n, wide),
      floatingActionButton: signedIn
          ? FloatingActionButton.extended(
              onPressed: () {
                if (wide) {
                  setState(() {
                    _creating = true;
                    _editing = false;
                  });
                } else {
                  context.push(AppRoutes.brewNew);
                }
              },
              icon: const Icon(Icons.add_outlined),
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
          emptyHint: l10n.homeEmptyHint,
          isSelected: (brew) => wide && !_creating && brew.id == _selectedBrewId,
          onTap: (context, brew) {
            if (wide) {
              setState(() {
                _selectedBrewId = brew.id;
                _editing = false;
                _creating = false;
              });
            } else {
              context.push(AppRoutes.brewPath(brew.id));
            }
          },
          row: (context, brew, {required selected, required onTap}) => ListRow(
            title: brew.purchase.product.name,
            subtitle: _brewSubtitle(context, brew),
            trailing: Rating(value: brew.rating),
            archived: brew.isArchived,
            selected: selected,
            onTap: onTap,
          ),
        ),
        // 失敗の原因 (ネットワーク、500 など) に応じた文言にする。
        SessionStatus.unknown => Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(AppSpacing.x6),
            child: ErrorBanner(
              message: controller.unknownError == null
                  ? l10n.errorNetwork
                  : messageForError(controller.unknownError!, l10n),
              onRetry: _retry,
            ),
          ),
        ),
        _ => Center(
          child: Text(l10n.loading, style: AppTextStyle.body(color: BrewbookTheme.of(context).palette.inkMuted)),
        ),
      },
    );
  }

  /// 右の面 (詳細、編集、新規の登録)。
  Widget? _detailPane(BuildContext context) {
    if (_creating) {
      return BrewFormScreen(
        services: widget.services,
        embedded: true,
        onClose: () => setState(() => _creating = false),
        onSaved: () => setState(() => _creating = false),
      );
    }
    final selectedId = _selectedBrewId;
    if (selectedId == null) {
      return null;
    }
    if (_editing) {
      return BrewFormScreen(
        services: widget.services,
        id: selectedId,
        embedded: true,
        onClose: () => setState(() => _editing = false),
        onSaved: () => setState(() => _editing = false),
      );
    }
    return BrewDetailScreen(
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
        return WideLayout(railIndex: 0, list: list, detail: _detailPane(context));
      },
    );
  }
}

/// メニューのログアウトの項目の値。
const String _logoutMenuValue = 'logout';
