import 'package:coffee_log/api/api_client.dart';
import 'package:coffee_log/app.dart';
import 'package:coffee_log/auth/passkey_client.dart';
import 'package:coffee_log/download/file_download.dart';
import 'package:coffee_log/l10n/app_localizations.dart';
import 'package:coffee_log/photo/image_converter.dart';
import 'package:coffee_log/photo/photo_picker.dart';
import 'package:coffee_log/records/clock.dart';
import 'package:coffee_log/screens/home_screen.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:http/http.dart' as http;

/// アプリを立ち上げ、起動時のセッションの確認が終わるまで進める。
///
/// 表示の言語は端末の言語で変わるため (FR-16)、テストは既定で英語に固定する。
/// 日本語の解決は `locale_resolution_test.dart` が確認する。
/// 写真の選択と変換は実行環境に合う実装を既定にし、テストは偽の実装を差し込む (FR-10)。
Future<void> pumpApp(
  WidgetTester tester, {
  required ApiClient apiClient,
  required PasskeyClient passkeyClient,
  List<Locale> locales = const <Locale>[Locale('en')],
  PhotoPicker? photoPicker,
  ImageConverter? imageConverter,
  http.Client? photoUploadClient,
  DeviceClock? clock,
  FileDownload? fileDownload,
  List<NavigatorObserver> navigatorObservers = const <NavigatorObserver>[],
}) async {
  tester.platformDispatcher.localesTestValue = locales;
  addTearDown(tester.platformDispatcher.clearLocalesTestValue);
  await tester.pumpWidget(
    CoffeeLogApp(
      apiClient: apiClient,
      passkeyClient: passkeyClient,
      photoPicker: photoPicker,
      imageConverter: imageConverter,
      photoUploadClient: photoUploadClient,
      clock: clock,
      fileDownload: fileDownload,
      navigatorObservers: navigatorObservers,
    ),
  );
  await tester.pumpAndSettle();
}

/// テストが期待する文言を ARB から取る。
Future<AppLocalizations> loadL10n([Locale locale = const Locale('en')]) {
  return AppLocalizations.delegate.load(locale);
}

/// テストから経路を直接開く (ホームの上に積むため、戻る操作と保存後の pop が動く)。
Future<void> openLocation(WidgetTester tester, String location) async {
  GoRouter.of(tester.element(find.byType(HomeScreen))).push(location);
  await tester.pumpAndSettle();
}

/// 画面外にあるウィジェットを表示させてから、文字を入力する。
Future<void> scrollAndEnterText(WidgetTester tester, Finder finder, String text) async {
  await tester.ensureVisible(finder);
  await tester.pumpAndSettle();
  await tester.enterText(finder, text);
  await tester.pumpAndSettle();
}

/// 画面外にあるウィジェットを表示させてから押す。
Future<void> scrollAndTap(WidgetTester tester, Finder finder) async {
  await tester.ensureVisible(finder);
  await tester.pumpAndSettle();
  await tester.tap(finder);
  await tester.pumpAndSettle();
}
