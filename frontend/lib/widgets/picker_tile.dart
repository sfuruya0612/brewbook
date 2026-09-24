import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';

/// 参照を 1 つ選ぶ欄 (FR-9、FR-11)。
///
/// 選択はダイアログで行う。ダイアログは画面に数えない (PRD の成功指標) ため、
/// 参照の選択は画面ではなくダイアログで行う。
class PickerTile extends StatelessWidget {
  const PickerTile({
    super.key,
    required this.label,
    required this.value,
    required this.onPressed,
    this.errorText,
  });

  /// 項目名。
  final String label;

  /// 選ばれた値。選んでいないときは null。
  final String? value;

  /// 押したときに選択のダイアログを開く。null のときは押せない。
  final VoidCallback? onPressed;

  /// 検証の誤り (ARB から取る)。
  final String? errorText;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final error = errorText;
    return ListTile(
      title: Text(label),
      subtitle: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Text(value ?? l10n.unsetLabel),
          if (error != null)
            Text(error, style: TextStyle(color: Theme.of(context).colorScheme.error)),
        ],
      ),
      trailing: IconButton(
        tooltip: l10n.selectButton,
        icon: const Icon(Icons.chevron_right),
        onPressed: onPressed,
      ),
      onTap: onPressed,
    );
  }
}
