import 'package:flutter/widgets.dart';

import 'api/api_client.dart';
import 'app.dart';
import 'auth/passkey_client.dart';

/// アプリの入口。
///
/// API は同一オリジンの `/api` を呼び (ADR-0005)、パスキーは実行環境に合う実装を使う (ADR-0007)。
void main() {
  runApp(
    CoffeeLogApp(
      apiClient: ApiClient(),
      passkeyClient: createPasskeyClient(),
    ),
  );
}
