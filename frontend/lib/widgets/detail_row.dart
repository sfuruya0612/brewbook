import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';

/// 詳細の画面の 1 行 (項目名と値)。
class DetailRow extends StatelessWidget {
  const DetailRow({super.key, required this.label, required this.value, this.onTap});

  final String label;
  final String value;

  /// 押したときの動き。参照をたどるときに使う (UC-6)。無いときは押せない。
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      title: Text(label),
      subtitle: Text(value),
      trailing: onTap == null ? null : const Icon(Icons.chevron_right),
      onTap: onTap,
    );
  }
}

/// 値が無いときは「未設定」を返す。
String valueOrUnset(String? value, AppLocalizations l10n) {
  if (value == null || value.isEmpty) {
    return l10n.unsetLabel;
  }
  return value;
}
