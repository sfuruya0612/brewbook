import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';

/// 1 から 5 の評価を丸で示す (docs/design/components/Rating)。
///
/// 星は使わない。一覧と詳細は 10 px の丸、入力は [RatingInput] の 28 px を使う。
class Rating extends StatelessWidget {
  const Rating({super.key, required this.value, this.showValue = false, this.dotSize = 10});

  /// 評価。未評価のときは null。
  final int? value;

  /// 「4 / 5」を右に添えるか。無いときは丸だけ。
  final bool showValue;

  /// 丸の大きさ (px)。一覧と詳細は 10。
  final double dotSize;

  @override
  Widget build(BuildContext context) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final int? rating = value;
    final double gap = dotSize * 0.4;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        for (int index = 1; index <= 5; index++)
          Padding(
            padding: EdgeInsets.only(right: index == 5 ? 0 : gap),
            child: Container(
              width: dotSize,
              height: dotSize,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: rating != null && index <= rating ? brewbook.crema : brewbook.line,
              ),
            ),
          ),
        if (showValue && rating != null) ...<Widget>[
          const SizedBox(width: AppSpacing.x1),
          Text(
            AppLocalizations.of(context).ratingValue(rating.toString()),
            style: AppTextStyle.caption(color: brewbook.palette.inkMuted),
          ),
        ],
      ],
    );
  }
}

/// 評価の入力 (docs/design/components/Rating)。
///
/// 28 px の丸を 5 つ並べ、押した丸までを塗る。未評価のときは 5 つとも枠だけにし、
/// 「未評価」を添える。評価は任意なので既定値を入れない (FR-11)。
class RatingInput extends StatelessWidget {
  const RatingInput({super.key, required this.value, required this.onChanged, this.enabled = true});

  /// 現在の評価。未評価のときは null。
  final int? value;

  /// 評価が変わったときの処理。同じ値を押しても選択を外さない。
  final ValueChanged<int> onChanged;

  /// 押せるか。
  final bool enabled;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final int? rating = value;
    return Row(
      children: <Widget>[
        for (int index = 1; index <= 5; index++)
          Padding(
            padding: const EdgeInsets.only(right: AppSpacing.x1),
            child: Semantics(
              button: true,
              label: l10n.ratingValue(index.toString()),
              child: GestureDetector(
                onTap: enabled ? () => onChanged(index) : null,
                child: Container(
                  width: 28,
                  height: 28,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    color: rating != null && index <= rating
                        ? brewbook.crema
                        : Colors.transparent,
                    border: Border.all(
                      color: rating != null && index <= rating
                          ? brewbook.crema
                          : brewbook.lineStrong,
                    ),
                  ),
                ),
              ),
            ),
          ),
        const SizedBox(width: AppSpacing.x2),
        if (rating == null)
          Text(l10n.ratingNone, style: AppTextStyle.caption(color: brewbook.inkFaint))
        else
          Text(
            l10n.ratingValue(rating.toString()),
            style: AppTextStyle.value(color: brewbook.palette.inkMuted),
          ),
      ],
    );
  }
}
