import 'package:flutter/material.dart';

import '../theme/app_theme.dart';
import '../theme/tokens.dart';

/// フォームの枠 (docs/design の `.form`)。
///
/// 画面の左右は 16 px、下は 32 px、入力欄の間は 20 px 空ける。
class AppForm extends StatelessWidget {
  const AppForm({super.key, required this.children});

  /// 上から並べる入力欄。
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(
        AppSpacing.x4,
        AppSpacing.x4,
        AppSpacing.x4,
        AppSpacing.x8,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          for (int index = 0; index < children.length; index++) ...<Widget>[
            if (index > 0) const SizedBox(height: AppSpacing.x5),
            children[index],
          ],
        ],
      ),
    );
  }
}

/// 2 列に並べる入力欄の行 (docs/design の `.grid2`)。
class AppFormRow extends StatelessWidget {
  const AppFormRow({super.key, required this.children, this.flex = const <int>[]});

  /// 並べる欄。1 つか 2 つ。
  final List<Widget> children;

  /// 欄の幅の比。無いときは等分にする。
  final List<int> flex;

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        for (int index = 0; index < children.length; index++) ...<Widget>[
          if (index > 0) const SizedBox(width: AppSpacing.x3),
          Expanded(
            flex: index < flex.length ? flex[index] : 1,
            child: children[index],
          ),
        ],
      ],
    );
  }
}

/// セクションの見出し (label、ink-muted)。
class SectionLabel extends StatelessWidget {
  const SectionLabel({super.key, required this.text});

  /// 見出しの文言 (ARB から取る)。
  final String text;

  @override
  Widget build(BuildContext context) {
    return Text(
      text,
      style: AppTextStyle.label(color: BrewbookTheme.of(context).palette.inkMuted),
    );
  }
}
