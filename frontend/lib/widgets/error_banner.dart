import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';

/// 画面に出すエラーのバナー (docs/design/components/Feedback)。
///
/// signal-soft の地に signal の文字、radius-md。通信エラーと 401 以外の API エラーは
/// 右端に「再試行」(signal の 600) を置く。文言は ARB から取ったものを渡す (FR-16)。
class ErrorBanner extends StatelessWidget {
  const ErrorBanner({super.key, required this.message, this.onRetry});

  /// 表示する文言。
  final String message;

  /// 再試行の処理。渡さないときは再試行のボタンを出さない。
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    final VoidCallback? retry = onRetry;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: AppSpacing.x4, vertical: AppSpacing.x3),
      decoration: BoxDecoration(
        color: brewbook.palette.signalSoft,
        borderRadius: AppRadius.mdAll,
      ),
      child: Row(
        children: <Widget>[
          Expanded(
            child: Text(message, style: AppTextStyle.body(color: brewbook.palette.signal)),
          ),
          if (retry != null) ...<Widget>[
            const SizedBox(width: AppSpacing.x3),
            TextButton(
              onPressed: retry,
              style: TextButton.styleFrom(
                foregroundColor: brewbook.palette.signal,
                textStyle: AppTextStyle.body(color: brewbook.palette.signal).copyWith(
                  fontWeight: FontWeight.w600,
                ),
                minimumSize: const Size(0, 40),
                padding: const EdgeInsets.symmetric(horizontal: AppSpacing.x3),
              ),
              child: Text(l10n.retryButton),
            ),
          ],
        ],
      ),
    );
  }
}
