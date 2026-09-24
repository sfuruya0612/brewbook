import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:http/http.dart' as http;

import 'api/api_client.dart';
import 'auth/auth_controller.dart';
import 'auth/auth_scope.dart';
import 'auth/passkey_client.dart';
import 'l10n/app_localizations.dart';
import 'l10n/locale_resolution.dart';
import 'photo/image_converter.dart';
import 'photo/photo_picker.dart';
import 'records/record_services.dart';
import 'router/app_router.dart';

/// アプリのルート。
///
/// API クライアントとパスキーのクライアントは差し替えられる (テストが使う)。パスキーの
/// クライアントは実行環境に合う実装を [createPasskeyClient] が返す (ADR-0007)。
/// 写真の選択と変換も同じく実行環境に合う実装を既定にし、テストは偽の実装を差し込む。
class CoffeeLogApp extends StatefulWidget {
  const CoffeeLogApp({
    super.key,
    required this.apiClient,
    required this.passkeyClient,
    this.photoPicker,
    this.imageConverter,
    this.photoUploadClient,
    this.navigatorObservers = const <NavigatorObserver>[],
  });

  final ApiClient apiClient;
  final PasskeyClient passkeyClient;

  /// 写真のファイルの選択 (FR-10)。無いときは実行環境に合う実装を使う。
  final PhotoPicker? photoPicker;

  /// 写真の JPEG への変換 (FR-10)。無いときは実行環境に合う実装を使う。
  final ImageConverter? imageConverter;

  /// R2 への PUT に使う HTTP のクライアント (テストが差し替える)。
  final http.Client? photoUploadClient;

  /// ルーターの監視 (画面数の成功指標を測るテストが渡す。PRD の成功指標)。
  final List<NavigatorObserver> navigatorObservers;

  @override
  State<CoffeeLogApp> createState() => _CoffeeLogAppState();
}

class _CoffeeLogAppState extends State<CoffeeLogApp> {
  late final AuthController _controller;
  late final RecordServices _services;
  late final GoRouter _router;

  @override
  void initState() {
    super.initState();
    _controller = AuthController(
      apiClient: widget.apiClient,
      passkeyClient: widget.passkeyClient,
    );
    _services = RecordServices(
      apiClient: widget.apiClient,
      photoPicker: widget.photoPicker,
      imageConverter: widget.imageConverter,
      photoUploadClient: widget.photoUploadClient,
    );
    // 401 の応答でセッションが失われたときは、ログイン画面へ遷移させる (ADR-0007)。
    widget.apiClient.onUnauthorized = _controller.markSignedOut;
    _router = createAppRouter(
      _controller,
      _services,
      observers: widget.navigatorObservers,
    );
    // 起動時に `GET /api/passkeys` を呼び、ログイン状態を判定する。
    _controller.check();
  }

  @override
  void dispose() {
    _router.dispose();
    _services.dispose();
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
