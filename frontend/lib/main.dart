import 'package:flutter/widgets.dart';
import 'package:flutter_web_plugins/url_strategy.dart';

import 'api/api_client.dart';
import 'app.dart';
import 'auth/passkey_client.dart';

/// アプリの入口。
///
/// API は同一オリジンの `/api` を呼び (ADR-0005)、パスキーは実行環境に合う実装を使う (ADR-0007)。
void main() {
  // Flutter のルーティングのパスをブラウザのパスに出す (既定のハッシュ形式にしない)。
  // Worker の Static Assets は一致する静的ファイルの無いパスに `index.html` を 200 で返すため
  // (not_found_handling)、登録用リンク (`/register?token=<トークン>`) を直接開いても画面が出る (ADR-0004、ADR-0005)。
  usePathUrlStrategy();
  runApp(
    BrewBookApp(
      apiClient: ApiClient(),
      passkeyClient: createPasskeyClient(),
    ),
  );
}
