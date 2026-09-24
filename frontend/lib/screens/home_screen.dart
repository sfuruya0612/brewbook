import 'package:flutter/material.dart';

import '../auth/auth_controller.dart';
import '../auth/auth_scope.dart';
import '../l10n/app_localizations.dart';
import '../widgets/error_banner.dart';
import '../widgets/error_message.dart';

/// ホームの画面 (抽出の一覧)。一覧の中身は 0014 が作る。
///
/// 起動時のセッションの確認の結果を表示し、確認できなかったときは再試行を促す (ADR-0007)。
class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  /// 直近の失敗の文言 (ARB から取る)。
  String? _errorMessage;
  bool _busy = false;

  Future<void> _logout() async {
    final l10n = AppLocalizations.of(context);
    final controller = AuthScope.read(context);
    setState(() {
      _busy = true;
      _errorMessage = null;
    });
    try {
      await controller.logout();
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() => _errorMessage = messageForError(error, l10n));
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _retry() => AuthScope.read(context).check();

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final controller = AuthScope.of(context);
    return Scaffold(
      appBar: AppBar(
        title: Text(l10n.homeTitle),
        actions: <Widget>[
          if (controller.status == SessionStatus.signedIn)
            TextButton(
              onPressed: _busy ? null : _logout,
              child: Text(l10n.logoutButton),
            ),
        ],
      ),
      body: Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              switch (controller.status) {
                SessionStatus.signedIn => Text(l10n.homeDescription),
                // 失敗の原因 (ネットワーク、500 など) に応じた文言にする。
                SessionStatus.unknown => ErrorBanner(
                  message: controller.unknownError == null
                      ? l10n.errorNetwork
                      : messageForError(controller.unknownError!, l10n),
                  onRetry: _retry,
                ),
                _ => Text(l10n.loading),
              },
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
