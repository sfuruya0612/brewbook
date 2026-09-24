import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'support/fake_api.dart';
import 'support/fake_passkey_client.dart';
import 'support/fake_records.dart';
import 'support/pump_app.dart';

/// カーソル方式の一覧 (FR-11、FR-12) のウィジェットテスト。
///
/// 50 件を超える記録にも到達できることを、ホームの抽出一覧で確認する (PRD の性能と想定規模)。
void main() {
  testWidgets('末尾までスクロールすると次のページ (既定 50 件) を読み込む', (tester) async {
    // 一覧の要求のカーソルを記録する (続きの要求がカーソルを運ぶことを確かめる)。
    final cursors = <String?>[];
    final api = signedInApi()
      ..onQuery('GET', '/api/brews', (query) {
        // 件数の既定は 50 件 (PRD の性能)。
        expect(query['limit'], '50');
        cursors.add(query['cursor']);
        if (query['cursor'] == null) {
          return (
            status: 200,
            body: pageJson(
              key: 'brews',
              items: <Map<String, Object?>>[
                for (int index = 1; index <= 50; index++)
                  brewJson(
                    id: 'brew-$index',
                    purchase: purchaseJson(product: productJson(name: 'Product $index')),
                  ),
              ],
              nextCursor: 'next-page',
            ),
          );
        }
        return (
          status: 200,
          body: pageJson(
            key: 'brews',
            items: <Map<String, Object?>>[
              brewJson(
                id: 'brew-51',
                purchase: purchaseJson(product: productJson(name: 'Product 51')),
              ),
            ],
          ),
        );
      });
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());

    expect(find.text('Product 1'), findsOneWidget);
    expect(find.text('Product 51'), findsNothing);

    // 末尾までスクロールすると、続きを読む (カーソル方式の一覧。ADR-0002)。
    await tester.scrollUntilVisible(
      find.text('Product 51'),
      300,
      scrollable: find.byType(Scrollable),
    );
    await tester.pumpAndSettle();

    expect(find.text('Product 51'), findsOneWidget);
    expect(cursors, <String?>[null, 'next-page']);
    expect(api.calls, <String>['GET /api/passkeys', 'GET /api/brews', 'GET /api/brews']);
  });

  testWidgets('続きが無いときは追加の要求をしない', (tester) async {
    final api = signedInApi()
      ..on(
        'GET',
        '/api/brews',
        status: 200,
        body: pageJson(
          key: 'brews',
          items: <Map<String, Object?>>[
            brewJson(id: 'brew-1', purchase: purchaseJson(product: productJson(name: 'Only'))),
          ],
        ),
      );
    await pumpApp(tester, apiClient: api.client(), passkeyClient: FakePasskeyClient());

    await tester.drag(find.byType(Scrollable), const Offset(0, -400));
    await tester.pumpAndSettle();

    expect(api.calls, <String>['GET /api/passkeys', 'GET /api/brews']);
  });
}
