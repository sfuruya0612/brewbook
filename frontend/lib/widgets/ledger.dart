import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';

/// 詳細の項目名と値の表 (docs/design/components/Ledger)。
///
/// 項目名は左に label、値は右揃え。行ごとに line の罫線を引く。
/// 未設定の項目は行を消さず「未設定」を ink-faint で出す。
class Ledger extends StatelessWidget {
  const Ledger({super.key, required this.rows});

  /// 表の行。
  final List<Widget> rows;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: rows,
    );
  }
}

/// [Ledger] の 1 行。
class LedgerRow extends StatelessWidget {
  const LedgerRow({
    super.key,
    required this.label,
    this.value,
    this.unit,
    this.mono = true,
    this.trailing,
  });

  /// 項目名 (ARB から取る)。
  final String label;

  /// 値。未設定のときは null。
  final String? value;

  /// 単位 (g、℃、秒、通貨コードなど)。caption の ink-muted で添える。
  final String? unit;

  /// 値を等幅で組むか。文字列の項目は false にする。
  final bool mono;

  /// 値の代わりに置く操作 (写真の差し替えと削除など)。
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final String? text = value;
    final Widget valueWidget = trailing ?? _value(context, l10n, brewbook, text);
    return Container(
      padding: const EdgeInsets.symmetric(vertical: AppSpacing.x3),
      decoration: BoxDecoration(
        border: Border(bottom: BorderSide(color: brewbook.line)),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          Padding(
            padding: const EdgeInsets.only(right: AppSpacing.x6),
            child: Text(label, style: AppTextStyle.label(color: brewbook.palette.inkMuted)),
          ),
          Expanded(child: Align(alignment: Alignment.centerRight, child: valueWidget)),
        ],
      ),
    );
  }

  Widget _value(
    BuildContext context,
    AppLocalizations l10n,
    BrewbookTheme brewbook,
    String? text,
  ) {
    if (text == null || text.isEmpty) {
      return Text(l10n.unsetLabel, style: AppTextStyle.body(color: brewbook.inkFaint));
    }
    final TextStyle valueStyle = mono
        ? AppTextStyle.value(color: brewbook.palette.ink)
        : AppTextStyle.body(color: brewbook.palette.ink);
    final unitText = unit;
    final String? unitSuffix =
        unitText == null || unitText.isEmpty ? null : ' $unitText';
    return Text.rich(
      TextSpan(
        children: <InlineSpan>[
          TextSpan(text: text, style: valueStyle),
          if (unitSuffix != null)
            TextSpan(
              text: unitSuffix,
              style: AppTextStyle.caption(color: brewbook.palette.inkMuted),
            ),
        ],
      ),
      textAlign: TextAlign.end,
    );
  }
}

/// 値が無いときは「未設定」を返す (ARB の unsetLabel)。
String valueOrUnset(String? value, AppLocalizations l10n) {
  if (value == null || value.isEmpty) {
    return l10n.unsetLabel;
  }
  return value;
}
