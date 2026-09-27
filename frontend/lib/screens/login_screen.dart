import 'package:flutter/material.dart';

import '../auth/auth_scope.dart';
import '../l10n/app_localizations.dart';
import '../theme/app_theme.dart';
import '../theme/tokens.dart';
import '../widgets/brewbook_mark.dart';
import '../widgets/error_banner.dart';
import '../widgets/error_message.dart';

/// ログインの画面 (FR-2)。
///
/// パスキーだけでログインするため、利用者名とパスワードの入力は置かない。画面の中央に
/// 印と名前と説明と主要ボタンを置き、下端に登録用リンクの案内を出す (AppBar は無い)。
class LoginScreen extends StatefulWidget {
  const LoginScreen({super.key});

  @override
  State<LoginScreen> createState() => _LoginScreenState();
}

class _LoginScreenState extends State<LoginScreen> {
  /// 直近の失敗の文言 (ARB から取る)。
  String? _errorMessage;
  bool _busy = false;

  Future<void> _login() async {
    final l10n = AppLocalizations.of(context);
    final controller = AuthScope.read(context);
    setState(() {
      _busy = true;
      _errorMessage = null;
    });
    try {
      await controller.login();
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() => _errorMessage = loginErrorMessage(error, l10n));
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final BrewbookTheme brewbook = BrewbookTheme.of(context);
    return Scaffold(
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: AppSpacing.x6),
          child: Column(
            children: <Widget>[
              Expanded(
                child: Center(
                  child: SingleChildScrollView(
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: <Widget>[
                        const Center(child: BrewbookMark(width: 72)),
                        const SizedBox(height: AppSpacing.x4),
                        Text(
                          l10n.homeTitle,
                          textAlign: TextAlign.center,
                          style: AppTextStyle.wordmark(color: brewbook.palette.ink),
                        ),
                        const SizedBox(height: AppSpacing.x4),
                        Text(
                          l10n.loginDescription,
                          textAlign: TextAlign.center,
                          style: AppTextStyle.body(color: brewbook.palette.inkMuted),
                        ),
                        const SizedBox(height: AppSpacing.x8),
                        FilledButton.icon(
                          onPressed: _busy ? null : _login,
                          icon: const Icon(Icons.key_outlined, size: 20),
                          label: Text(l10n.loginButton),
                        ),
                        if (_errorMessage != null) ...<Widget>[
                          const SizedBox(height: AppSpacing.x4),
                          ErrorBanner(message: _errorMessage!),
                        ],
                      ],
                    ),
                  ),
                ),
              ),
              Padding(
                padding: const EdgeInsets.only(bottom: AppSpacing.x6, top: AppSpacing.x4),
                child: Text(
                  l10n.loginRegisterHint,
                  textAlign: TextAlign.center,
                  style: AppTextStyle.caption(color: brewbook.palette.inkMuted),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
