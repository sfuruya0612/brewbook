import 'package:flutter/material.dart';

import '../auth/auth_scope.dart';
import '../l10n/app_localizations.dart';
import '../widgets/error_banner.dart';
import '../widgets/error_message.dart';

/// ログインの画面 (FR-2)。
///
/// パスキーだけでログインするため、利用者名とパスワードの入力は置かない。
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
    return Scaffold(
      appBar: AppBar(title: Text(l10n.loginTitle)),
      body: Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              Text(l10n.loginDescription),
              const SizedBox(height: 24),
              FilledButton(
                onPressed: _busy ? null : _login,
                child: Text(l10n.loginButton),
              ),
              if (_busy) ...<Widget>[
                const SizedBox(height: 16),
                const Center(child: CircularProgressIndicator()),
              ],
              if (_errorMessage != null) ...<Widget>[
                const SizedBox(height: 16),
                ErrorBanner(message: _errorMessage!),
              ],
            ],
          ),
        ),
      ),
    );
  }
}
