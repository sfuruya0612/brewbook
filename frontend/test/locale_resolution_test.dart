import 'package:coffee_log/l10n/app_localizations.dart';
import 'package:coffee_log/l10n/locale_resolution.dart';
import 'package:coffee_log/screens/login_screen.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/fake_api.dart';
import 'support/fake_passkey_client.dart';
import 'support/pump_app.dart';

void main() {
  test('端末またはブラウザの言語が日本語なら日本語を選ぶ', () {
    expect(
      resolveLocale(const Locale('ja'), AppLocalizations.supportedLocales),
      const Locale('ja'),
    );
    expect(
      resolveLocale(const Locale('ja', 'JP'), AppLocalizations.supportedLocales),
      const Locale('ja'),
    );
  });

  test('日本語以外は英語を選ぶ', () {
    expect(
      resolveLocale(const Locale('en', 'US'), AppLocalizations.supportedLocales),
      const Locale('en'),
    );
    expect(
      resolveLocale(const Locale('fr'), AppLocalizations.supportedLocales),
      const Locale('en'),
    );
    expect(resolveLocale(null, AppLocalizations.supportedLocales), const Locale('en'));
  });

  testWidgets('端末の言語が日本語なら日本語の UI を表示する', (tester) async {
    final l10n = await loadL10n(const Locale('ja'));
    final api = FakeApi()..on('GET', '/api/passkeys', status: 401, body: unauthorizedBody);
    await pumpApp(
      tester,
      apiClient: api.client(),
      passkeyClient: FakePasskeyClient(),
      locales: const <Locale>[Locale('ja', 'JP')],
    );

    expect(find.byType(LoginScreen), findsOneWidget);
    expect(find.text(l10n.loginButton), findsOneWidget);
    expect(l10n.loginButton, 'パスキーでログイン');
  });

  testWidgets('端末の言語が日本語以外なら英語の UI を表示する', (tester) async {
    final l10n = await loadL10n();
    final api = FakeApi()..on('GET', '/api/passkeys', status: 401, body: unauthorizedBody);
    await pumpApp(
      tester,
      apiClient: api.client(),
      passkeyClient: FakePasskeyClient(),
      locales: const <Locale>[Locale('en', 'US')],
    );

    expect(find.byType(LoginScreen), findsOneWidget);
    expect(find.text(l10n.loginButton), findsOneWidget);
    expect(l10n.loginButton, 'Log in with a passkey');
  });

  testWidgets('日本語以外の言語は英語の UI を表示する', (tester) async {
    final l10n = await loadL10n();
    final api = FakeApi()..on('GET', '/api/passkeys', status: 401, body: unauthorizedBody);
    await pumpApp(
      tester,
      apiClient: api.client(),
      passkeyClient: FakePasskeyClient(),
      locales: const <Locale>[Locale('fr', 'FR')],
    );

    expect(find.text(l10n.loginButton), findsOneWidget);
  });
}
