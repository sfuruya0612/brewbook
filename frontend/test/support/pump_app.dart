import 'package:coffee_log/api/api_client.dart';
import 'package:coffee_log/app.dart';
import 'package:coffee_log/auth/passkey_client.dart';
import 'package:coffee_log/l10n/app_localizations.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

/// アプリを立ち上げ、起動時のセッションの確認が終わるまで進める。
///
/// 表示の言語は端末の言語で変わるため (FR-16)、テストは既定で英語に固定する。
/// 日本語の解決は `locale_resolution_test.dart` が確認する。
Future<void> pumpApp(
  WidgetTester tester, {
  required ApiClient apiClient,
  required PasskeyClient passkeyClient,
  List<Locale> locales = const <Locale>[Locale('en')],
}) async {
  tester.platformDispatcher.localesTestValue = locales;
  addTearDown(tester.platformDispatcher.clearLocalesTestValue);
  await tester.pumpWidget(
    CoffeeLogApp(apiClient: apiClient, passkeyClient: passkeyClient),
  );
  await tester.pumpAndSettle();
}

/// テストが期待する文言を ARB から取る。
Future<AppLocalizations> loadL10n([Locale locale = const Locale('en')]) {
  return AppLocalizations.delegate.load(locale);
}
