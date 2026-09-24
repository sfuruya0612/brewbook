import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';

/// 画面に出すエラーの表示。文言は ARB から取ったものを渡す (FR-16)。
class ErrorBanner extends StatelessWidget {
  const ErrorBanner({super.key, required this.message, this.onRetry});

  /// 表示する文言。
  final String message;

  /// 再試行の処理。渡さないときは再試行のボタンを出さない。
  final VoidCallback? onRetry;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final colors = Theme.of(context).colorScheme;
    return Card(
      color: colors.errorContainer,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Text(message, style: TextStyle(color: colors.onErrorContainer)),
            if (onRetry != null) ...<Widget>[
              const SizedBox(height: 8),
              TextButton(
                onPressed: onRetry,
                child: Text(l10n.retryButton),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
