import 'package:flutter/material.dart';

import '../theme/app_theme.dart';
import '../theme/tokens.dart';

/// 参照先 (購入、商品、店) をたどるタイル (docs/design/components/ReferenceTile)。
///
/// roast-soft の地に種類 (caption) と名前 (heading) を置き、右端に「>」を付ける。
/// 押すとその記録の詳細へ移る (UC-6)。
class ReferenceTile extends StatelessWidget {
  const ReferenceTile({super.key, required this.kind, required this.name, this.onTap});

  /// 種類 (「購入」「商品」「店」。ARB から取る)。
  final String kind;

  /// たどる先の名前。
  final String name;

  /// 押したときの動き。無いときは押せない。
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    return Material(
      color: brewbook.roastSoft,
      borderRadius: AppRadius.mdAll,
      child: InkWell(
        onTap: onTap,
        borderRadius: AppRadius.mdAll,
        child: Padding(
          padding: const EdgeInsets.symmetric(
            horizontal: AppSpacing.x4,
            vertical: AppSpacing.x3,
          ),
          child: Row(
            children: <Widget>[
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(kind, style: AppTextStyle.caption(color: brewbook.palette.inkMuted)),
                    const SizedBox(height: 2),
                    Text(
                      name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: AppTextStyle.body(color: brewbook.palette.ink).copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: AppSpacing.x3),
              Icon(Icons.chevron_right_outlined, size: 20, color: brewbook.palette.inkMuted),
            ],
          ),
        ),
      ),
    );
  }
}

/// 参照先のタイルを縦に並べる連鎖。
class ReferenceChain extends StatelessWidget {
  const ReferenceChain({super.key, required this.tiles});

  /// たどる順のタイル。
  final List<Widget> tiles;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        for (int index = 0; index < tiles.length; index++) ...<Widget>[
          if (index > 0) const SizedBox(height: AppSpacing.x2),
          tiles[index],
        ],
      ],
    );
  }
}
