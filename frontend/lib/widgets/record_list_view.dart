import 'package:flutter/material.dart';

import '../api/models.dart';
import '../l10n/app_localizations.dart';
import '../records/record_services.dart';
import 'error_banner.dart';
import 'error_message.dart';

/// 一覧の 1 ページを読む関数 (カーソル方式。ADR-0002)。
typedef RecordPageLoader<T> =
    Future<RecordPage<T>> Function({String? cursor, required bool includeArchived});

/// 末尾までの残りの高さがこの値 (px) を下回ったら、次のページを読み込む。
const double loadMoreThreshold = 200;

/// カーソル方式の一覧の共通の枠 (FR-12)。
///
/// アーカイブ済みを含める切り替え、末尾までのスクロールでの追加読み込み、行ごとのアーカイブと
/// アーカイブ解除を持つ。中身の表示は [title] と [subtitle] が決める。
class RecordListView<T> extends StatefulWidget {
  const RecordListView({
    super.key,
    required this.load,
    required this.services,
    required this.title,
    this.subtitle,
    this.onTap,
    this.isArchived,
    this.setArchived,
    this.showArchivedToggle = true,
  });

  /// 1 ページを読む。
  final RecordPageLoader<T> load;

  /// 記録の変更の通知 (登録、更新、アーカイブ) を受け取って読み直す。
  final RecordServices services;

  /// 行の主な表示。
  final Widget Function(BuildContext context, T item) title;

  /// 行の補足の表示。無いときは null を返す。
  final Widget? Function(BuildContext context, T item)? subtitle;

  /// 行を押したときの動き。無いときは押せない。
  final void Function(BuildContext context, T item)? onTap;

  /// アーカイブ済みかを返す。アーカイブの操作を置かないときは null。
  final bool Function(T item)? isArchived;

  /// アーカイブとアーカイブ解除を行う。null のときは行にボタンを置かない。
  final Future<void> Function(T item, bool archived)? setArchived;

  /// アーカイブ済みを含める切り替えを置くか (選択のダイアログでは置かない)。
  final bool showArchivedToggle;

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

  /// 行のアーカイブとアーカイブ解除を行う (FR-12)。
  Future<void> _toggleArchived(T item) async {
    final archived = widget.isArchived?.call(item) ?? false;
    final l10n = AppLocalizations.of(context);
    try {
      await widget.setArchived!(item, !archived);
      widget.services.markRecordsChanged();
    } catch (error) {
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(messageForError(error, l10n))),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Column(
      children: <Widget>[
        if (widget.showArchivedToggle)
          SwitchListTile(
            title: Text(l10n.includeArchivedLabel),
            value: _includeArchived,
            onChanged: (value) {
              setState(() => _includeArchived = value);
              _load(reset: true);
            },
          ),
        if (_errorMessage != null)
          Padding(
            padding: const EdgeInsets.all(8),
            child: ErrorBanner(
              message: _errorMessage!,
              onRetry: () => _load(reset: true),
            ),
          ),
        if (_loading) const LinearProgressIndicator(),
        Expanded(
          child: _items.isEmpty
              ? Center(child: Text(_loaded ? l10n.noRecords : l10n.loading))
              : NotificationListener<ScrollNotification>(
                  onNotification: _onScroll,
                  child: ListView.builder(
                    itemCount: _items.length,
                    itemBuilder: (context, index) => _row(context, _items[index]),
                  ),
                ),
        ),
      ],
    );
  }

  /// 1 行を組み立てる。アーカイブの操作は行の末尾に置く。
  Widget _row(BuildContext context, T item) {
    final l10n = AppLocalizations.of(context);
    final subtitle = widget.subtitle;
    final onTap = widget.onTap;
    final setArchived = widget.setArchived;
    return ListTile(
      title: widget.title(context, item),
      subtitle: subtitle == null ? null : subtitle(context, item),
      trailing: setArchived == null
          ? null
          : IconButton(
              tooltip: (widget.isArchived?.call(item) ?? false)
                  ? l10n.unarchiveButton
                  : l10n.archiveButton,
              icon: Icon(
                (widget.isArchived?.call(item) ?? false)
                    ? Icons.unarchive_outlined
                    : Icons.archive_outlined,
              ),
              onPressed: () => _toggleArchived(item),
            ),
      onTap: onTap == null ? null : () => onTap(context, item),
    );
  }
}
