import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';
import 'app_field.dart';

/// 参照を 1 つ選ぶ欄 (FR-9、FR-11)。
///
/// [AppField] と同じ枠 (paper-sunken の地、line-strong の枠、radius-sm) に種類と
/// 選択中の名前、右端に「>」を置く。未選択はプレースホルダーを ink-faint で出す。
/// 選択はダイアログで行う (ダイアログは画面に数えない。PRD の成功指標)。
class PickerTile extends StatelessWidget {
  const PickerTile({
    super.key,
    required this.label,
    required this.value,
    required this.onPressed,
    this.placeholder,
    this.caption,
    this.required = false,
    this.errorText,
  });

  /// 項目名 (種類)。ARB から取る。
  final String label;

  /// 選ばれた値。選んでいないときは null。
  final String? value;

  /// 押したときに選択のダイアログを開く。null のときは押せない。
  final VoidCallback? onPressed;

  /// 未選択のときの文言 (「購入を選ぶ」など)。無いときは ARB の unsetLabel。
  final String? placeholder;

  /// 選択中の名前の下に出す補足 (購入日 / 店など)。
  final String? caption;

  /// 必須の項目か。
  final bool required;

  /// 検証の誤り (ARB から取る)。
  final String? errorText;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final bool hasError = errorText != null;
    final bool isSet = value != null;
    final String? error = errorText;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Material(
          color: brewbook.palette.paperSunken,
          borderRadius: AppRadius.smAll,
          child: InkWell(
            onTap: onPressed,
            borderRadius: AppRadius.smAll,
            child: Container(
              constraints: const BoxConstraints(minHeight: 48),
              padding: const EdgeInsets.symmetric(
                horizontal: AppSpacing.x3,
                vertical: AppSpacing.x2,
              ),
              decoration: BoxDecoration(
                borderRadius: AppRadius.smAll,
                border: Border.all(
                  color: hasError || !isSet && required
                      ? brewbook.palette.signal
                      : brewbook.lineStrong,
                ),
              ),
              child: Row(
                children: <Widget>[
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      mainAxisSize: MainAxisSize.min,
                      children: <Widget>[
                        Row(
                          children: <Widget>[
                            Text(
                              label,
                              style: AppTextStyle.caption(color: brewbook.palette.inkMuted),
                            ),
                            if (required) ...<Widget>[
                              const SizedBox(width: AppSpacing.x1),
                              Text(
                                l10n.requiredLabel,
                                style: AppTextStyle.caption(
                                  color: brewbook.palette.inkMuted,
                                ).copyWith(fontWeight: FontWeight.w400),
                              ),
                            ],
                          ],
                        ),
                        const SizedBox(height: 2),
                        Text(
                          value ?? placeholder ?? l10n.unsetLabel,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: AppTextStyle.body(
                            color: isSet ? brewbook.palette.ink : brewbook.inkFaint,
                          ).copyWith(fontWeight: isSet ? FontWeight.w600 : FontWeight.w400),
                        ),
                        if (caption != null) ...<Widget>[
                          const SizedBox(height: 2),
                          Text(
                            caption!,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: AppTextStyle.caption(color: brewbook.palette.inkMuted),
                          ),
                        ],
                      ],
                    ),
                  ),
                  const SizedBox(width: AppSpacing.x3),
                  Icon(
                    Icons.chevron_right_outlined,
                    size: 20,
                    color: brewbook.palette.inkMuted,
                  ),
                ],
              ),
            ),
          ),
        ),
        if (error != null) ...<Widget>[
          const SizedBox(height: AppSpacing.x1 + 2),
          Text(error, style: AppTextStyle.caption(color: brewbook.palette.signal)),
        ],
      ],
    );
  }
}
