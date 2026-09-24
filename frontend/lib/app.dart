import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

import 'api/api_client.dart';
import 'auth/auth_controller.dart';
import 'auth/auth_scope.dart';
import 'auth/passkey_client.dart';
import 'l10n/app_localizations.dart';
import 'l10n/locale_resolution.dart';
import 'router/app_router.dart';

/// アプリのルート。
///
/// API クライアントとパスキーのクライアントは差し替えられる (テストが使う)。パスキーの
/// クライアントは実行環境に合う実装を [createPasskeyClient] が返す (ADR-0007)。
class CoffeeLogApp extends StatefulWidget {
  const CoffeeLogApp({super.key, required this.apiClient, required this.passkeyClient});

  final ApiClient apiClient;
  final PasskeyClient passkeyClient;

  @override
  State<CoffeeLogApp> createState() => _CoffeeLogAppState();
}

class _CoffeeLogAppState extends State<CoffeeLogApp> {
  late final AuthController _controller;
  late final GoRouter _router;

  @override
  void initState() {
    super.initState();
    _controller = AuthController(
      apiClient: widget.apiClient,
      passkeyClient: widget.passkeyClient,
    );
    // 401 の応答でセッションが失われたときは、ログイン画面へ遷移させる (ADR-0007)。
    widget.apiClient.onUnauthorized = _controller.markSignedOut;
    _router = createAppRouter(_controller);
    // 起動時に `GET /api/passkeys` を呼び、ログイン状態を判定する。
    _controller.check();
  }

  @override
  void dispose() {
    _router.dispose();
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AuthScope(
      controller: _controller,
      child: MaterialApp.router(
        onGenerateTitle: (context) => AppLocalizations.of(context).appTitle,
        // 端末またはブラウザの言語が日本語なら日本語、それ以外は英語にする (FR-16)。
        localeResolutionCallback: resolveLocale,
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        routerConfig: _router,
      ),
    );
  }
}
