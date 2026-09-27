import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';

/// 項目名を上に置く入力欄の枠 (docs/design/components/Field)。
///
/// 浮動ラベルは使わない。項目名は label の ink-muted、必須は「必須」を太さ 400 で
/// 項目名の横に添える。検証の誤りは枠の下に caption の signal で書く。
class AppField extends StatelessWidget {
  const AppField({
    super.key,
    required this.label,
    this.required = false,
    this.errorText,
    this.helperText,
    required this.child,
  });

  /// 項目名 (ARB から取る)。
  final String label;

  /// 必須の項目か。任意の項目には何も付けない。
  final bool required;

  /// 検証の誤り (ARB から取る)。
  final String? errorText;

  /// 入力の助け (ARB から取る)。誤りがあるときは出さない。
  final String? helperText;

  /// 入力欄。
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final String? error = errorText;
    final String? helper = helperText;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Row(
          children: <Widget>[
            Text(label, style: AppTextStyle.label(color: brewbook.palette.inkMuted)),
            if (required) ...<Widget>[
              const SizedBox(width: AppSpacing.x1),
              Text(
                l10n.requiredLabel,
                style: AppTextStyle.label(color: brewbook.palette.inkMuted).copyWith(
                  fontWeight: FontWeight.w400,
                ),
              ),
            ],
          ],
        ),
        const SizedBox(height: 6),
        child,
        if (error != null) ...<Widget>[
          const SizedBox(height: AppSpacing.x1 + 2),
          Text(error, style: AppTextStyle.caption(color: brewbook.palette.signal)),
        ] else if (helper != null) ...<Widget>[
          const SizedBox(height: AppSpacing.x1 + 2),
          Text(helper, style: AppTextStyle.caption(color: brewbook.palette.inkMuted)),
        ],
      ],
    );
  }
}

/// [AppField] の中に置く 1 行の入力欄。
///
/// 数値と日付は等幅 ([mono]) で組み、単位は右端に caption で添える。
/// 検証の誤りは枠を signal にし、理由は [AppField] が下に書く。
class AppTextField extends StatelessWidget {
  const AppTextField({
    super.key,
    required this.controller,
    required this.label,
    this.required = false,
    this.enabled = true,
    this.errorText,
    this.helperText,
    this.hintText,
    this.unit,
    this.mono = true,
    this.keyboardType,
    this.maxLines = 1,
    this.minLines,
    this.autofocus = false,
    this.suffixIcon,
    this.onSubmitted,
  });

  final TextEditingController controller;
  final String label;
  final bool required;
  final bool enabled;
  final String? errorText;
  final String? helperText;
  final String? hintText;

  /// 右端に添える単位 (g、℃、秒)。無いときは添えない。
  final String? unit;

  /// 値を等幅で組むか。数値と日付は true。
  final bool mono;

  final TextInputType? keyboardType;
  final int? maxLines;
  final int? minLines;
  final bool autofocus;

  /// 右端のアイコン (カレンダーなど)。単位より優先する。
  final Widget? suffixIcon;

  final ValueChanged<String>? onSubmitted;

  @override
  Widget build(BuildContext context) {
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final bool hasError = errorText != null;
    final String? unitText = unit;
    final Widget? suffix = suffixIcon ??
        (unitText == null
            ? null
            : Padding(
                padding: const EdgeInsets.only(left: AppSpacing.x1),
                child: Text(
                  unitText,
                  style: AppTextStyle.caption(color: brewbook.palette.inkMuted),
                ),
              ));
    return AppField(
      label: label,
      required: required,
      errorText: errorText,
      helperText: helperText,
      child: TextField(
        controller: controller,
        enabled: enabled,
        keyboardType: keyboardType,
        maxLines: maxLines,
        minLines: minLines,
        autofocus: autofocus,
        onSubmitted: onSubmitted,
        style: mono
            ? AppTextStyle.value(color: brewbook.palette.ink)
            : AppTextStyle.body(color: brewbook.palette.ink),
        decoration: InputDecoration(
          hintText: hintText,
          suffixIcon: suffix,
          suffixIconConstraints: const BoxConstraints(minWidth: 0, minHeight: 0),
          enabledBorder: hasError
              ? OutlineInputBorder(
                  borderRadius: AppRadius.smAll,
                  borderSide: BorderSide(color: brewbook.palette.signal),
                )
              : null,
          focusedBorder: hasError
              ? OutlineInputBorder(
                  borderRadius: AppRadius.smAll,
                  borderSide: BorderSide(color: brewbook.palette.signal),
                )
              : null,
        ),
      ),
    );
  }
}
