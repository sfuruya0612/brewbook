import 'package:flutter/material.dart';

import '../api/models.dart';
import '../l10n/app_localizations.dart';
import '../records/record_services.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';
import 'brewbook_mark.dart';
import 'error_banner.dart';
import 'error_message.dart';

/// 一覧の 1 ページを読む関数 (カーソル方式。ADR-0002)。
typedef RecordPageLoader<T> =
    Future<RecordPage<T>> Function({String? cursor, required bool includeArchived});

/// 一覧の 1 行を組み立てる関数。
///
/// [selected] は広い画面で選択中の行か (roast-soft の地)、[onTap] は行を押したときの動き。
typedef RecordRowBuilder<T> =
    Widget Function(BuildContext context, T item, {required bool selected, required VoidCallback onTap});

/// 末尾までの残りの高さがこの値 (px) を下回ったら、次のページを読み込む。
const double loadMoreThreshold = 200;

/// カーソル方式の一覧の共通の枠 (FR-12)。
///
/// アーカイブ済みを含める切り替え、末尾までのスクロールでの追加読み込みを持つ。
/// 中身の表示は [row] が決める。行ごとのアーカイブは詳細の画面から行う。
class RecordListView<T> extends StatefulWidget {
  const RecordListView({
    super.key,
    required this.load,
    required this.services,
    required this.row,
    this.isSelected,
    this.onTap,
    this.showArchivedToggle = true,
    this.emptyHint,
  });

  /// 1 ページを読む。
  final RecordPageLoader<T> load;

  /// 記録の変更の通知 (登録、更新、アーカイブ) を受け取って読み直す。
  final RecordServices services;

  /// 行の組み立て。
  final RecordRowBuilder<T> row;

  /// 広い画面で選択中の行か。
  final bool Function(T item)? isSelected;

  /// 行を押したときの動き。無いときは押せない。
  final void Function(BuildContext context, T item)? onTap;

  /// アーカイブ済みを含める切り替えを置くか (選択のダイアログでは置かない)。
  final bool showArchivedToggle;

  /// 記録が無いときに、次にすることを示す 1 文 (ARB から取る)。
  final String? emptyHint;

  @override
  State<RecordListView<T>> createState() => _RecordListViewState<T>();
}

class _RecordListViewState<T> extends State<RecordListView<T>> {
  final List<T> _items = <T>[];
  String? _nextCursor;
  bool _includeArchived = false;
  bool _loading = false;

  /// 読み込み中に届いた読み直しの要求。
  bool _pendingReset = false;
  bool _loaded = false;
  String? _errorMessage;

  @override
  void initState() {
    super.initState();
    widget.services.revision.addListener(_onRecordsChanged);
    _load(reset: true);
  }

  @override
  void dispose() {
    widget.services.revision.removeListener(_onRecordsChanged);
    super.dispose();
  }

  /// 記録が変わったので、先頭から読み直す。
  void _onRecordsChanged() {
    if (mounted) {
      _load(reset: true);
    }
  }

  /// ページを読む。[reset] が true のときは先頭から読み直す。
  Future<void> _load({required bool reset}) async {
    if (_loading) {
      // 読み込み中に届いた読み直しの要求は、完了後に実行する (古い表示を残さない)。
      if (reset) {
        _pendingReset = true;
      }
      return;
    }
    setState(() {
      _loading = true;
      if (reset) {
        _items.clear();
        _nextCursor = null;
        _errorMessage = null;
      }
    });
    try {
      final page = await widget.load(
        cursor: reset ? null : _nextCursor,
        includeArchived: _includeArchived,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _items.addAll(page.items);
        _nextCursor = page.nextCursor;
        _loaded = true;
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() => _errorMessage = messageForError(error, AppLocalizations.of(context)));
    } finally {
      if (mounted) {
        setState(() => _loading = false);
      }
      if (_pendingReset) {
        _pendingReset = false;
        await _load(reset: true);
      }
    }
  }

  /// 末尾まで来たら次のページを読む (50 件を超える記録にも到達できる。PRD の成功指標)。
  bool _onScroll(ScrollNotification notification) {
    if (notification.metrics.extentAfter < loadMoreThreshold) {
      _loadMore();
    }
    return false;
  }

  /// 続きがあれば次のページを読む。
  void _loadMore() {
    if (_loading || _nextCursor == null) {
      return;
    }
    _load(reset: false);
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final String? errorMessage = _errorMessage;
    return Column(
      children: <Widget>[
        if (widget.showArchivedToggle)
          Container(
            padding: const EdgeInsets.symmetric(
              horizontal: AppSpacing.x4,
              vertical: AppSpacing.x2,
            ),
            decoration: BoxDecoration(
              border: Border(bottom: BorderSide(color: brewbook.line)),
            ),
            child: Row(
              children: <Widget>[
                Transform.scale(
                  scale: 0.85,
                  child: Switch(
                    value: _includeArchived,
                    onChanged: (value) {
                      setState(() => _includeArchived = value);
                      _load(reset: true);
                    },
                  ),
                ),
                const SizedBox(width: AppSpacing.x2),
                Text(
                  l10n.includeArchivedLabel,
                  style: AppTextStyle.label(color: brewbook.palette.inkMuted),
                ),
              ],
            ),
          ),
        if (errorMessage != null)
          Padding(
            padding: const EdgeInsets.all(AppSpacing.x4),
            child: ErrorBanner(
              message: errorMessage,
              onRetry: () => _load(reset: true),
            ),
          ),
        Expanded(
          child: _items.isEmpty
              ? (_loaded
                    ? EmptyState(message: l10n.noRecords, hint: widget.emptyHint)
                    : Center(
                        child: Text(
                          l10n.loading,
                          style: AppTextStyle.body(color: brewbook.palette.inkMuted),
                        ),
                      ))
              : NotificationListener<ScrollNotification>(
                  onNotification: _onScroll,
                  child: ListView.builder(
                    itemCount: _items.length + (_loading && _nextCursor != null ? 1 : 0),
                    itemBuilder: (context, index) {
                      if (index >= _items.length) {
                        // 続きの読み込み中は最後の行の下に「読み込み中」を caption で出す。
                        return Padding(
                          padding: const EdgeInsets.all(AppSpacing.x4),
                          child: Center(
                            child: Text(
                              l10n.loading,
                              style: AppTextStyle.caption(color: brewbook.palette.inkMuted),
                            ),
                          ),
                        );
                      }
                      final item = _items[index];
                      return widget.row(
                        context,
                        item,
                        selected: widget.isSelected?.call(item) ?? false,
                        onTap: () => widget.onTap?.call(context, item),
                      );
                    },
                  ),
                ),
        ),
      ],
    );
  }
}
