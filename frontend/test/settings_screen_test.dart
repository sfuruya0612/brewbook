import 'dart:convert';

import 'package:coffee_log/records/values.dart';
import 'package:coffee_log/router/app_router.dart';
import 'package:coffee_log/screens/login_screen.dart';
import 'package:coffee_log/screens/settings_screen.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/fake_api.dart';
import 'support/fake_file_download.dart';
import 'support/fake_passkey_client.dart';
import 'support/fake_records.dart';
import 'support/pump_app.dart';

/// 設定の画面 (FR-3、FR-4、FR-14、FR-15) のウィジェットテスト。
void main() {
  const String createdAt1 = '2026-09-01T01:00:00.000Z';
  const String createdAt2 = '2026-09-02T02:00:00.000Z';
  const String usedAt1 = '2026-09-10T10:00:00.000Z';

  /// パスキーの応答 1 件 (FR-3)。
  Map<String, Object?> passkeyJson({
    required String id,
    required String name,
    String createdAt = createdAt1,
    String? lastUsedAt,
  }) {
    return <String, Object?>{
      'id': id,
      'name': name,
      'created_at': createdAt,
      'last_used_at': lastUsedAt,
    };
  }

  /// パスキーの一覧の応答 (FR-3)。
  Map<String, Object?> passkeyList(List<Map<String, Object?>> passkeys) {
    return <String, Object?>{'passkeys': passkeys};
  }

  /// ホームと設定の画面の応答を登録した API を組み立てる。
  FakeApi settingsApi({required Map<String, Object?> passkeys}) {
    return signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(key: 'brews', items: <Map<String, Object?>>[]),
      )
      ..on('GET', '/api/passkeys', status: 200, body: passkeys);
  }

  /// ログイン済みで設定の画面を開いた状態にする。
  Future<void> openSettings(
    WidgetTester tester, {
    required FakeApi api,
    FakeFileDownload? fileDownload,
    FakePasskeyClient? passkeyClient,
  }) async {
    await pumpApp(
      tester,
      apiClient: api.client(),
      passkeyClient: passkeyClient ?? FakePasskeyClient(),
      fileDownload: fileDownload ?? FakeFileDownload(),
    );
    await openLocation(tester, AppRoutes.settings);
    expect(find.byType(SettingsScreen), findsOneWidget);
  }

  testWidgets('パスキーの一覧に名前と登録日時と最終使用日時を表示する', (tester) async {
    final l10n = await loadL10n();
    const locale = Locale('en');
    final api = settingsApi(
      passkeys: passkeyList(<Map<String, Object?>>[
        passkeyJson(id: 'passkey-1', name: 'Home PC', lastUsedAt: usedAt1),
        passkeyJson(id: 'passkey-2', name: 'Phone', createdAt: createdAt2),
      ]),
    );
    await openSettings(tester, api: api);

    expect(find.text('Home PC'), findsOneWidget);
    expect(find.text('Phone'), findsOneWidget);
    // 登録日時と最終使用日時を端末の言語で表示する (FR-3、FR-16)。
    expect(
      find.textContaining(l10n.passkeyCreatedAt(displayTimestamp(createdAt1, locale))),
      findsOneWidget,
    );
    expect(
      find.textContaining(l10n.passkeyLastUsedAt(displayTimestamp(usedAt1, locale))),
      findsOneWidget,
    );
    // まだ使われていないパスキーはその旨を表示する (FR-3)。
    expect(find.textContaining(l10n.passkeyNotUsedYet), findsOneWidget);
    // 一覧は設定の画面を開いたときに読む (FR-3)。
    expect(api.calls, <String>[
      'GET /api/passkeys',
      'GET /api/brews',
      'GET /api/passkeys',
    ]);
  });

  testWidgets('パスキーを追加する', (tester) async {
    final l10n = await loadL10n();
    final passkeys = FakePasskeyClient();
    var added = false;
    final api = settingsApi(passkeys: passkeyList(const <Map<String, Object?>>[]))
      ..onQuery(
        'GET',
        '/api/passkeys',
        (query) => (
          status: 200,
          body: passkeyList(<Map<String, Object?>>[
            if (added) passkeyJson(id: 'passkey-2', name: 'Phone', createdAt: createdAt2),
          ]),
        ),
      )
      ..on(
        'POST',
        '/api/passkeys/begin',
        status: 200,
        body: <String, Object?>{'challenge': 'test-challenge'},
      )
      ..onQuery(
        'POST',
        '/api/passkeys/complete',
        (query) {
          added = true;
          return (
            status: 200,
            body: passkeyJson(id: 'passkey-2', name: 'Phone', createdAt: createdAt2),
          );
        },
      );
    await openSettings(tester, api: api, passkeyClient: passkeys);

    await tester.tap(find.widgetWithText(OutlinedButton, l10n.addPasskeyButton));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), ' Phone ');
    await tester.tap(find.widgetWithText(FilledButton, l10n.addButton));
    await tester.pumpAndSettle();

    // 名前は前後の空白を除いて送る (ADR-0004)。
    expect(
      api.lastBody('POST', '/api/passkeys/complete'),
      containsPair('name', 'Phone'),
    );
    // 作成のオプションはそのままパスキーのクライアントへ渡す (FR-3)。
    expect(passkeys.createdOptions?['challenge'], 'test-challenge');
    // 追加の後は一覧を読み直す (FR-3)。
    expect(find.text('Phone'), findsOneWidget);
    expect(api.calls, <String>[
      'GET /api/passkeys',
      'GET /api/brews',
      'GET /api/passkeys',
      'POST /api/passkeys/begin',
      'POST /api/passkeys/complete',
      'GET /api/passkeys',
    ]);
  });

  testWidgets('名前が空のままでは追加の API を呼ばない', (tester) async {
    final l10n = await loadL10n();
    final api = settingsApi(passkeys: passkeyList(const <Map<String, Object?>>[]));
    await openSettings(tester, api: api);

    await tester.tap(find.widgetWithText(OutlinedButton, l10n.addPasskeyButton));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, l10n.addButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.passkeyNameError), findsOneWidget);
    expect(api.calls, isNot(contains('POST /api/passkeys/begin')));
  });

  testWidgets('パスキーの名前を変更する', (tester) async {
    final l10n = await loadL10n();
    var renamed = false;
    final api = settingsApi(
      passkeys: passkeyList(<Map<String, Object?>>[
        passkeyJson(id: 'passkey-1', name: 'Home PC'),
      ]),
    )
      ..onQuery(
        'PATCH',
        '/api/passkeys/passkey-1',
        (query) {
          renamed = true;
          return (
            status: 200,
            body: passkeyJson(id: 'passkey-1', name: 'Desktop'),
          );
        },
      )
      ..onQuery(
        'GET',
        '/api/passkeys',
        (query) => (
          status: 200,
          body: passkeyList(<Map<String, Object?>>[
            passkeyJson(id: 'passkey-1', name: renamed ? 'Desktop' : 'Home PC'),
          ]),
        ),
      );
    await openSettings(tester, api: api);

    await tester.tap(find.byTooltip(l10n.renameButton));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'Desktop');
    await tester.tap(find.widgetWithText(FilledButton, l10n.renameButton));
    await tester.pumpAndSettle();

    expect(
      api.lastBody('PATCH', '/api/passkeys/passkey-1'),
      <String, Object?>{'name': 'Desktop'},
    );
    expect(find.text('Desktop'), findsOneWidget);
    expect(find.text('Home PC'), findsNothing);
  });

  testWidgets('パスキーを削除する', (tester) async {
    final l10n = await loadL10n();
    var deleted = false;
    final api = settingsApi(
      passkeys: passkeyList(<Map<String, Object?>>[
        passkeyJson(id: 'passkey-1', name: 'Home PC'),
        passkeyJson(id: 'passkey-2', name: 'Phone', createdAt: createdAt2),
      ]),
    )
      ..onQuery(
        'DELETE',
        '/api/passkeys/passkey-1',
        (query) {
          deleted = true;
          return (status: 200, body: <String, Object?>{'id': 'passkey-1'});
        },
      )
      ..onQuery(
        'GET',
        '/api/passkeys',
        (query) => (
          status: 200,
          body: passkeyList(<Map<String, Object?>>[
            if (!deleted) passkeyJson(id: 'passkey-1', name: 'Home PC'),
            passkeyJson(id: 'passkey-2', name: 'Phone', createdAt: createdAt2),
          ]),
        ),
      );
    await openSettings(tester, api: api);

    await tester.tap(find.byTooltip(l10n.deleteButton).first);
    await tester.pumpAndSettle();

    expect(api.calls, contains('DELETE /api/passkeys/passkey-1'));
    expect(find.text('Home PC'), findsNothing);
    expect(find.text('Phone'), findsOneWidget);
  });

  testWidgets('パスキーが 1 つしかないときは削除を無効にする', (tester) async {
    final l10n = await loadL10n();
    final api = settingsApi(
      passkeys: passkeyList(<Map<String, Object?>>[
        passkeyJson(id: 'passkey-1', name: 'Home PC'),
      ]),
    );
    await openSettings(tester, api: api);

    // 最後の 1 つは削除できない (設計判断)。
    final deleteButton = tester.widget<IconButton>(
      find.widgetWithIcon(IconButton, Icons.delete_outline),
    );
    expect(deleteButton.onPressed, isNull);
    expect(find.byTooltip(l10n.deleteButton), findsOneWidget);
    expect(api.calls, isNot(contains('DELETE /api/passkeys/passkey-1')));
  });

  testWidgets('サーバーが 409 を返したら最後の 1 つを消せないことを表示する', (tester) async {
    final l10n = await loadL10n();
    // 別の端末でパスキーが消えた場合など、画面の一覧が 2 つでもサーバーは 409 を返し得る (FR-3)。
    final api = settingsApi(
      passkeys: passkeyList(<Map<String, Object?>>[
        passkeyJson(id: 'passkey-1', name: 'Home PC'),
        passkeyJson(id: 'passkey-2', name: 'Phone', createdAt: createdAt2),
      ]),
    )..on('DELETE', '/api/passkeys/passkey-1', status: 409, body: conflictBody);
    await openSettings(tester, api: api);

    await tester.tap(find.byTooltip(l10n.deleteButton).first);
    await tester.pumpAndSettle();

    expect(find.text(l10n.passkeyLastDeleteError), findsOneWidget);
    expect(find.text('Home PC'), findsOneWidget);
  });

  testWidgets('パスキーの読み込みの失敗は再試行できる', (tester) async {
    final l10n = await loadL10n();
    var calls = 0;
    final api = settingsApi(passkeys: passkeyList(const <Map<String, Object?>>[]))
      ..onQuery(
        'GET',
        '/api/passkeys',
        (query) {
          calls += 1;
          // 1 回目は起動時のセッションの確認 (FR-2)。
          if (calls == 1) {
            return (status: 200, body: passkeyList(const <Map<String, Object?>>[]));
          }
          if (calls == 2) {
            return (
              status: 500,
              body: <String, Object?>{
                'error': <String, Object?>{'code': 'internal_error', 'message': 'temporary'},
              },
            );
          }
          return (
            status: 200,
            body: passkeyList(<Map<String, Object?>>[
              passkeyJson(id: 'passkey-1', name: 'Home PC'),
            ]),
          );
        },
      );
    await openSettings(tester, api: api);

    expect(find.text(l10n.errorUnexpected), findsOneWidget);
    await tester.tap(find.widgetWithText(TextButton, l10n.retryButton));
    await tester.pumpAndSettle();

    expect(find.text('Home PC'), findsOneWidget);
  });

  testWidgets('エクスポートの JSON をファイルとしてダウンロードする', (tester) async {
    final l10n = await loadL10n();
    final download = FakeFileDownload();
    final exportBody = <String, Object?>{
      'shops': <Object?>[],
      'products': <Object?>[],
      'flavor_tags': <Object?>[],
      'product_flavor_tags': <Object?>[],
      'purchases': <Object?>[],
      'brews': <Object?>[],
    };
    final api = settingsApi(passkeys: passkeyList(const <Map<String, Object?>>[]))
      ..on('GET', '/api/export', status: 200, body: exportBody);
    await openSettings(tester, api: api, fileDownload: download);

    await scrollAndTap(tester, find.widgetWithText(OutlinedButton, l10n.exportButton));
    await tester.pumpAndSettle();

    // 応答の JSON をそのままファイルとして保存する (FR-14)。
    expect(api.calls, contains('GET /api/export'));
    expect(download.saved, hasLength(1));
    final saved = download.saved.single;
    expect(saved.fileName, 'coffee-log-export.json');
    expect(utf8.decode(saved.bytes), jsonEncode(exportBody));
  });

  testWidgets('エクスポートの失敗は再試行を促す表示にする', (tester) async {
    final l10n = await loadL10n();
    final download = FakeFileDownload();
    final api = settingsApi(passkeys: passkeyList(const <Map<String, Object?>>[]))
      ..onNetworkError('GET', '/api/export');
    await openSettings(tester, api: api, fileDownload: download);

    await scrollAndTap(tester, find.widgetWithText(OutlinedButton, l10n.exportButton));
    await tester.pumpAndSettle();

    expect(find.text(l10n.errorNetwork), findsOneWidget);
    expect(download.saved, isEmpty);
  });

  testWidgets('確認のボタンを押さなければアカウント削除の API を呼ばない', (tester) async {
    final l10n = await loadL10n();
    final api = settingsApi(
      passkeys: passkeyList(<Map<String, Object?>>[
        passkeyJson(id: 'passkey-1', name: 'Home PC'),
      ]),
    );
    await openSettings(tester, api: api);

    // 取り消しでは削除しない (FR-15)。
    await scrollAndTap(tester, find.widgetWithText(FilledButton, l10n.deleteAccountButton));
    await tester.pumpAndSettle();
    expect(find.text(l10n.deleteAccountConfirmMessage), findsOneWidget);
    await tester.tap(find.widgetWithText(TextButton, l10n.cancelButton));
    await tester.pumpAndSettle();
    expect(find.byType(SettingsScreen), findsOneWidget);
    expect(api.calls, isNot(contains('DELETE /api/account')));

    // ダイアログの外を押して閉じても削除しない (FR-15)。
    await scrollAndTap(tester, find.widgetWithText(FilledButton, l10n.deleteAccountButton));
    await tester.pumpAndSettle();
    await tester.tapAt(const Offset(10, 10));
    await tester.pumpAndSettle();
    expect(find.byType(SettingsScreen), findsOneWidget);
    expect(find.byType(LoginScreen), findsNothing);
    expect(api.calls, isNot(contains('DELETE /api/account')));
  });

  testWidgets('確認を押すとアカウントを削除し、ログイン画面へ遷移する', (tester) async {
    final l10n = await loadL10n();
    final api = settingsApi(
      passkeys: passkeyList(<Map<String, Object?>>[
        passkeyJson(id: 'passkey-1', name: 'Home PC'),
      ]),
    )..on('DELETE', '/api/account', status: 204, body: null);
    await openSettings(tester, api: api);

    await scrollAndTap(tester, find.widgetWithText(FilledButton, l10n.deleteAccountButton));
    await tester.pumpAndSettle();
    await tester.tap(find.widgetWithText(FilledButton, l10n.deleteButton));
    await tester.pumpAndSettle();

    expect(api.calls, contains('DELETE /api/account'));
    // 削除の後はログインの状態が消え、ログイン画面へ遷移する (FR-15)。
    expect(find.byType(LoginScreen), findsOneWidget);
    expect(find.byType(SettingsScreen), findsNothing);
  });

  testWidgets('ログアウトするとログイン画面へ遷移する', (tester) async {
    final l10n = await loadL10n();
    final api = settingsApi(
      passkeys: passkeyList(<Map<String, Object?>>[
        passkeyJson(id: 'passkey-1', name: 'Home PC'),
      ]),
    )..on('POST', '/api/auth/logout', status: 200, body: <String, Object?>{});
    await openSettings(tester, api: api);

    await scrollAndTap(tester, find.widgetWithText(OutlinedButton, l10n.logoutButton));
    await tester.pumpAndSettle();

    expect(api.calls, contains('POST /api/auth/logout'));
    expect(find.byType(LoginScreen), findsOneWidget);
  });

  testWidgets('ホームのメニューから設定の画面を開く', (tester) async {
    final l10n = await loadL10n();
    final api = settingsApi(
      passkeys: passkeyList(<Map<String, Object?>>[
        passkeyJson(id: 'passkey-1', name: 'Home PC'),
      ]),
    );
    await pumpApp(
      tester,
      apiClient: api.client(),
      passkeyClient: FakePasskeyClient(),
      fileDownload: FakeFileDownload(),
    );

    await tester.tap(find.byTooltip(l10n.menuTooltip));
    await tester.pumpAndSettle();
    await tester.tap(find.text(l10n.settingsTitle).last);
    await tester.pumpAndSettle();

    expect(find.byType(SettingsScreen), findsOneWidget);
    expect(find.text('Home PC'), findsOneWidget);
  });
}
