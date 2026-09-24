# Flutter の基盤と認証の画面を作る

Created: 2026-09-21
Model: deepseek-v4p1-flash
Completed: 2026-09-24
対応 ADR: ADR-0007 (docs/adr/0007-frontend-flutter-web-first.md)、ADR-0004 (docs/adr/0004-passkey-only-authentication.md)、ADR-0005 (docs/adr/0005-same-origin-deployment.md)
関連 PRD: FR-1、FR-2、FR-4、FR-16、制約と前提 (Flutter の構成とテスト)
依存: 0002, 0005

## 背景

ADR-0007 は、Frontend を Flutter で書き、Web を先行して iOS はビルドできる状態を保つと決めた。
ルーティングは go_router、状態管理は Flutter 標準の ChangeNotifier と ValueNotifier、Web のレンダラは CanvasKit とし、追加の状態管理ライブラリは使わない。
UI の文言は ARB ファイルで日本語と英語を持ち、コードに直書きしない。
パスキーは Web では `navigator.credentials` を JS 相互運用で呼び、iOS は配布形態が決まってから実装する。
PRD の FR-16 は、端末の言語が日本語なら日本語、それ以外なら英語で表示することと、UI の文言を全て ARB に置くことと、日本語 UI の項目名の訳語を定める。
0005 で認証の API ができた。本 issue がアプリの基盤と認証の画面を作る。

## 目的

Flutter Web アプリの基盤 (ルーティング、翻訳、API クライアント、パスキーの相互運用) を作り、登録、ログイン、ログアウトができるようにする。

## 設計判断

- `frontend/` に Flutter のプロジェクトを作る。
  パッケージ名は `coffee_log` とし、プラットフォームは Web と iOS を作る (Android は対象外。PRD のやらないこと)。
  iOS 向けのプラットフォーム固有の実装はインターフェースの後ろに置き、CI では iOS をビルドしない (ADR-0007)。
- ルーティングは go_router を使い、`/login`、`/register`、`/` (ホーム) などの経路を台帳として 1 か所に置く (経路は 0014 以降が追加する)。
- 状態管理は ChangeNotifier と ValueNotifier だけを使い、追加のライブラリを入れない (ADR-0007)。
- 翻訳は Flutter の l10n (ARB) を使い、`lib/l10n/app_ja.arb` と `app_en.arb` を置く。
  言語の解決は端末またはブラウザの言語が日本語なら日本語、それ以外は英語とする (FR-16)。
  日本語 UI の項目名は、Producer を生産者、Origin を生産国、Region を地域、Process を精製方法、Variety を品種、Roast を焙煎度、Roast Date を焙煎日、Flavor Notes をフレーバーノートとする。
- UI の文言の直書きを防ぐ静的検査を Dart のテストとして置く。
  `lib/` の UI コードで Text、TextSpan、InputDecoration の labelText と hintText と helperText と errorText、Tooltip の message、SnackBar の content、AppBar の title に文字列リテラルを直接渡していないことを検証する (ログ、テストコード、翻訳キー、URL は対象外。FR-16)。
  日本語の ARB に 8 つの訳語が値として存在することも検証する。
  採らない案: 別のリントツールの追加 (Dart のテストで足りる)。
- API クライアントは `package:http` で `/api` の下を呼ぶ。
  応答のエラーの JSON (`{"error": {"code": ..., "message": ...}}`) を共通の型に変換し、401 はログイン画面へ遷移させ、ネットワークエラーは再試行を促す表示にする (ADR-0007 のオンライン前提)。
  オフラインでの記録と同期は作らない (PRD のやらないこと)。
- セッションはブラウザの Cookie が管理する (同一オリジン。ADR-0005)。
  起動時に `GET /api/passkeys` を呼び、401 ならログイン画面へ遷移してログイン状態を判定する (専用のセッション確認 API は設けない)。
- パスキーは `PasskeyClient` のインターフェースの後ろに置く。
  Web の実装は `dart:js_interop` と `package:web` で `navigator.credentials` を呼び、サーバーのオプションの JSON を変換する。
  iOS の実装は配布形態が決まってから足す (ADR-0007)。
- 追加する依存は `http` (API 呼び出し)、`web` (JS 相互運用) と、開発用の `flutter_lints` とする。
  `dio` などの HTTP クライアント、Riverpod などの状態管理、その他のパッケージは入れない (依存を追加する理由はここに記す)。
- テストは 3 種類に分ける。
  Widget テストは `frontend/test/` に置き、`frontend:test` (`flutter test`) で実行する。
  Canvas を使うテストは `frontend/test/` に置き、`frontend:test-web` (`flutter test --platform chrome`) で実行する (0014 の画像変換のテストが使う)。
  統合テストは `frontend/integration_test/` に置き、`frontend:test-integration` (`flutter drive` と chromedriver) で実行する。
- chromedriver は `mise.toml` の `[tools]` に版を固定して追加する。
  CI では同じ版の Chrome for Testing をワークフローで指定して入れる。
  ブラウザ本体はリポジトリのツールチェーンに含められないため、CI の環境の依存としてワークフローで版を固定する例外とし、理由をここに記す (ADR-0009 の「同じ mise.toml を使う」はツールチェーンとタスクの共有を指す)。
- `mise.toml` に `frontend:setup`、`frontend:build`、`frontend:analyze`、`frontend:test`、`frontend:test-web`、`frontend:test-integration` を追加し、`check` に参加させる (0002 の設計に従う)。
  `frontend:build` は `flutter build web` とし、CanvasKit を指定する (指定の方法は Flutter 3.47.5 の CLI に従う)。
- 採らない案: Flutter の開発サーバーをローカル開発に使う (別オリジンになり、CORS を許可しない決定と `Origin` の検証に反する。ADR-0005)、iOS の実装を先に作る (配布形態が未定。ADR-0007)。

## 完了条件

- `frontend/` の Flutter プロジェクトが Web と iOS のプラットフォームを持ち、`mise run frontend:build` と `mise run frontend:analyze` が成功する。
- FR-1 の受け入れ基準のうちクライアントの分を満たす。
  `/register?token=<トークン>` を開くとパスキー登録の画面が表示される (ウィジェットテストで判定する)。
- FR-2 と FR-4 のクライアントの分を満たす。
  ログイン画面で利用者名やパスワードの入力が不要で、パスキーでログインでき、ログアウトできる。
- FR-16 の受け入れ基準を満たす。
  言語の解決、ARB への集約、直書きの静的検査、日本語の ARB の 8 つの訳語を満たす。
- ログイン、登録、ログアウト、エラー表示 (400 と 401 とネットワークエラー) のウィジェットテストがある。
- `mise run frontend:test`、`mise run frontend:test-web`、`mise run frontend:test-integration` が成功し、統合テストのハーネスが動く (1 本のスモークテスト)。
- `mise run check` が通過する。

## 関連

- 0002 が `check` と CI の枠を作る。
- 0005 が認証の API を作る。
- 0014 が記録の画面を作り、画面数の成功指標の統合テストを追加する。
- 0015 と 0016 が統計と設定の画面を作る。
- 0017 がビルド成果物を Static Assets として配信する。

## 実装詳細の乖離

方式は変えず、実装の詳細として次を選んだ。

1. 統合テストの起動方法を `flutter drive ... -d chrome` から `-d web-server --browser-name=chrome` に変更し、タスク内で `chromedriver --port=4444` を起動 (`trap` で停止) するようにした。`flutter drive` は chromedriver を自分で起動せず、`-d chrome` はこの環境で「Synced 139.1MB.」「Caching compiled dill」の後 240 秒以上停止してセッションを作らないことを 2 回確認したためである (方針の方式は flutter drive と chromedriver とスモークテスト 1 本のままで、`-d web-server` では約 35 秒で成功する)。
2. chromedriver の版を `154.0.8037.57` とした。手元の Chrome は `154.0.8037.58` だが、chromedriver と linux64 の Chrome for Testing の両方が揃う版が `154.0.8037.57` であり、同じメジャー版なので動く (ビルド番号の違いは許容される)。CI も同じ版を入れ、`/usr/local/bin/google-chrome` にリンクして `CHROME_EXECUTABLE` を設定する (ブラウザ本体だけをワークフローで固定する例外。理由はコメントに記載)。
3. `frontend:test-integration` は複数行のシェルスクリプト (`run = """..."""`) とし、chromedriver の起動と停止を 1 つのタスクで完結させた。
4. `flutter build web` の `cupertino_icons` の警告は追加せず、警告のままとした (ビルドは成功する)。`pubspec.yaml` の依存は、方針が名指しした `http`、`web`、`flutter_lints` に加えて、方針が要求するルーティング (`go_router`)、l10n (`flutter_localizations`、`intl`)、統合テスト (`integration_test`) だけである (グラフの `fl_chart` は 0015 が追加する)。`frontend/lib/l10n/app_localizations*.dart` (ビルド時に再生成される) は `.gitignore` の対象外のためリポジトリに含める。
5. CanvasKit は Flutter 3.47.5 の CLI に指定のオプションが無く、JavaScript ビルドの既定のレンダラが CanvasKit であるため、`frontend:build` は `flutter build web` のままとした (ビルド結果の `flutter_bootstrap.js` の `buildConfig` が `renderer: canvaskit` であることを確認)。
6. パスキーの相互運用のうち JS との変換 (base64url のパディング、例外の種別) は、関数を `@visibleForTesting` で公開し、Chrome で動く `frontend:test-web` のテスト (`test/web/passkey_interop_test.dart`) が確認する。`navigator.credentials` の呼び出し自体は仮想認証器を使う 0017 の統合テストが確認する。
7. パスキーの名前の文字数は、クライアントも Unicode のスカラー値で数える (`String.runes`)。Backend の `validate_passkey_name` と同じ数え方にするためである (Dart の `String.length` は UTF-16 の符号単位で、サロゲートペアを含む名前でずれる)。
8. 起動時の確認 (`GET /api/passkeys`) は 401 だけを「セッション無し」とし、500 などの 401 以外の API エラーはネットワークエラーと同じ「状態が分からない」として再試行を促す (原因の文言を表示する)。登録の 400 は、Backend が名前の不正とクレデンシャルの検証の失敗のどちらでも同じコードで返すため、名前だけを指す文言ではなく入力の確認を促す汎用の文言にした。

## 解決方法

`frontend/` に Flutter のプロジェクトを作り、ルーティング、翻訳、API クライアント、パスキーの相互運用、認証の画面、テスト、`mise.toml` と CI の配線を追加した。

- `frontend/lib/main.dart`、`app.dart`、`router/app_router.dart` (経路の台帳)、`api/api_client.dart` と `api_error.dart` (`package:http` で `/api` を呼び、エラーの JSON を共通の型に変換し、401 はログインへ、ネットワークエラーは再試行の表示にする)、`auth/` (`auth_controller`、`auth_scope`、`PasskeyClient` のインターフェースと Web 実装 `passkey_client_web` (`dart:js_interop` と `package:web` で `navigator.credentials`)、iOS 用の `passkey_client_stub`)、`l10n/` (`app_ja.arb`、`app_en.arb`、`locale_resolution.dart`)、`screens/` (`home`、`login`、`register`)、`widgets/` (`error_banner`、`error_message`) を追加した。
- `frontend/test/` にウィジェットテストと単体テスト (合計 42 テスト。`frontend:test`) と `test/web/canvas_test.dart` と `test/web/passkey_interop_test.dart` (Canvas と JS の変換は `frontend:test-web` で実行)、`frontend/integration_test/app_test.dart` と `test_driver/` に統合テストのハーネスを追加した。
- `frontend/web/` と `frontend/ios/` のプラットフォームを作った (iOS はビルドできる状態を保つ。Android は作らない)。
- `mise.toml` に `chromedriver` (154.0.8037.57) を `[tools]` に、`frontend:setup`、`frontend:build`、`frontend:analyze`、`frontend:test`、`frontend:test-web`、`frontend:test-integration` の 6 タスクを追加し、`test` と `check` に参加させた。
- `.github/workflows/ci.yml` に Chrome for Testing の固定版を入れるステップを追加した。

完了条件の検証:

- プロジェクトとビルド: `frontend/web/` と `frontend/ios/` があり、`mise run frontend:analyze` が「No issues found!」、`mise run frontend:build` が成功した (CanvasKit の `canvaskit.js` と `canvaskit.wasm` を確認)。
- FR-1 のクライアントの分: `frontend/test/register_screen_test.dart` の「/register?token= を開くと登録の画面を表示する」と「トークンが無いときはその旨を表示する」が確認した。
- FR-2 と FR-4 のクライアントの分: `frontend/test/login_screen_test.dart` の「利用者名とパスワードの入力は置かない」と「パスキーでログインするとホームへ遷移する」、`frontend/test/home_screen_test.dart` の「ログアウトするとログイン画面へ遷移する」が確認した。
- FR-16: `frontend/lib/l10n/locale_resolution.dart` と `frontend/test/locale_resolution_test.dart` (端末が日本語なら日本語、その他なら英語)、`frontend/test/l10n_check_test.dart` の「日本語の ARB に 8 つの訳語がある」と「lib の UI コードに文言を直書きしていない」が確認した (生産者、生産国、地域、精製方法、品種、焙煎度、焙煎日、フレーバーノート)。
- エラー表示: `login_screen_test.dart` が 400 と 401 とネットワークエラーを、`register_screen_test.dart` が 400 (検証の失敗) と 404 と 409 と 410 とネットワークエラーを、`home_screen_test.dart` が 500 (再試行) とネットワークエラーとログアウトを確認した。名前の境界 (50 文字ちょうど、空白のみ) も確認した。
- テストコマンド: `frontend:test` が 42 件、`frontend:test-web` が 44 件 (JS の変換の検査を含む)、`frontend:test-integration` がスモークテスト 1 件 (「起動するとログイン画面を表示する」) を成功させた。`mise run check` が exit 0 で通過した (作業ツリーでの所要は 1015 秒)。
- `mise run check`: exit 0 (ベースラインからの新たな失敗は無し)。

方針からの乖離 (方式は変えていない): issue の「## 実装詳細の乖離」1 から 4 にある (統合テストの起動方法の変更、chromedriver の版、タスクの形、`cupertino_icons` の警告と生成物の同梱)。
