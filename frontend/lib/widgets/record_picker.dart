import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import '../records/record_services.dart';
import 'record_list_view.dart';

/// 記録を 1 件選んだ結果。
///
/// 「選ばなかった」(取り消した) ことと、参照を外す選択 ([RecordChoice.cleared]) を区別する。
class RecordChoice<T> {
  const RecordChoice.selected(T selected) : value = selected;

  /// 参照を外す選択 (店を指定しないなど)。
  const RecordChoice.cleared() : value = null;

  /// 選んだ記録。参照を外す選択のときは null。
  final T? value;

  /// 参照を外す選択か。
  bool get isCleared => value == null;
}

/// 記録を 1 件選ぶダイアログ (FR-9、FR-11)。
///
/// 一覧はカーソル方式で、末尾までスクロールすると次のページを読む。ダイアログは画面に
/// 数えない (PRD の成功指標) ため、参照の選択は画面ではなくダイアログで行う。
/// 取り消したときは null を返す。
Future<RecordChoice<T>?> showRecordPicker<T>({
  required BuildContext context,
  required String title,
  required RecordServices services,
  required RecordPageLoader<T> load,
  required Widget Function(BuildContext context, T item) titleBuilder,
  Widget? Function(BuildContext context, T item)? subtitleBuilder,
  /// 参照を外す選択肢の文言。null のときは置かない。
  String? clearLabel,
}) {
  final l10n = AppLocalizations.of(context);
  return showDialog<RecordChoice<T>>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(title),
      content: SizedBox(
        width: 400,
        height: 400,
        child: RecordListView<T>(
          load: load,
          services: services,
          showArchivedToggle: false,
          title: titleBuilder,
          subtitle: subtitleBuilder,
          onTap: (context, item) =>
              Navigator.of(context).pop(RecordChoice<T>.selected(item)),
        ),
      ),
      actions: <Widget>[
        if (clearLabel != null)
          TextButton(
            onPressed: () => Navigator.of(context).pop(RecordChoice<T>.cleared()),
            child: Text(clearLabel),
          ),
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancelButton),
        ),
      ],
    ),
  );
}
