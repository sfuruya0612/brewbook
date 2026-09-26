# アプリ名と名前空間を brewbook に変更する

Created: 2026-09-26
Model: deepseek-v4p1-flash
Completed: 2026-09-26
対応 ADR: ADR-0010 (docs/adr/0010-app-name-and-namespace-brewbook.md)、ADR-0001、ADR-0005、ADR-0008
関連 PRD: 制約と前提 (アプリ名)、セキュリティ (RP ID、Origin、R2 の CORS)、運用 (デプロイ)
依存: 0021、0022

## 背景

PRD の「制約と前提」は 2026-09-21 に、アプリ名 coffee-log、Flutter のパッケージ名 coffee_log、Rust の 3 クレート (coffee_log、coffee_log_admin、coffee_log_core) を定めた。
2026-09-26 に ADR-0010 でアプリ名を brewbook、名前空間を brew_book に変更し、PRD の現在の値も更新済みである。
`log` はログ (logging) の名前空間と紛らわしく、記録のアプリの名前として分かりにくい (2026-09-26 に所有者が指摘)。コードと設定には旧名が残っている。

2026-09-26 に、当時の HEAD の追跡ファイルで `git grep -l -F` で数えた出現数は次の通り (ADR-0010 の追加前)。

| 文字列 | ファイル数 |
| --- | --- |
| coffee_log | 149 |
| coffee_log_core | 102 |
| coffee-log | 48 |
| coffee_log_admin | 20 |
| coffee-log-photos | 8 |
| coffee_log-fuzz、coffee_log_pbt | 各 2 |

名前は、Rust のパッケージ名と import、Worker の設定、D1 と R2 の名前、Flutter のパッケージ名と import、iOS の設定、ビルドの設定、README と PRD にまたがる。
画面のアプリ名 (`appTitle` と `homeTitle`)、エクスポートのファイル名 (`coffee-log-export.json`)、iOS の表示名 (`Coffee Log`) にも現れる (2026-09-26 に確認)。

## 目的

アプリ名と名前空間を brewbook に統一し、logging と紛らわしくない名前にする。

## 設計判断

ADR-0010 の対応表に従う。主な対応は次の通り。

| 対象 | 変更前 | 変更後 |
| --- | --- | --- |
| アプリ名 | coffee-log | brewbook |
| 利用者向けの Worker | coffee-log | brewbook |
| 管理者 Worker | coffee-log-admin | brewbook-admin |
| D1 データベース | coffee-log | brewbook |
| R2 バケット | coffee-log-photos | brewbook-photos |
| 利用者向けの Rust クレート | coffee_log (backend/coffee_log) | brew_book (backend/brew_book) |
| 管理者の Rust クレート | coffee_log_admin | brew_book_admin |
| 共有の Rust クレート | coffee_log_core | brew_book_core |
| PBT のクレート | coffee_log_pbt | brew_book_pbt |
| fuzz のクレート | coffee_log-fuzz | brew_book-fuzz |
| Flutter のパッケージ | coffee_log | brew_book |
| 画面のアプリ名 (ARB の appTitle と homeTitle) | coffee-log | brewbook |
| エクスポートのファイル名 | coffee-log-export.json | brewbook-export.json |
| iOS の表示名 (CFBundleDisplayName) | Coffee Log | Brew Book |
| iOS の CFBundleName | coffee_log | brew_book |
| iOS の bundle identifier | com.example.coffeeLog | com.example.brewBook |
| PRD | docs/prd/coffee-log.md | docs/prd/brewbook.md |

- 変更は 1 つの issue で一括して行う。クレート名、Worker 名、D1 の名前、Flutter のパッケージ名は同じ文字列を参照し合い、一部だけを変えるとビルドとテストが通らない。
- ディレクトリの移動は `git mv` を使い、backend 配下の 3 クレート (`backend/coffee_log`、`backend/coffee_log_admin`、`backend/coffee_log_core`) の配置は変えない (ADR-0001)。
- 変更するファイルの範囲は backend/、frontend/、`mise.toml`、`README.md`、`.gitignore`、`.github/`、`docs/prd/` とし、`docs/adr/0001-backend-rust-on-cloudflare-workers.md` の PRD 参照も更新する。主な対象は次の通り。
  - 各 `Cargo.toml` の `package.name`、workspace の `members`、依存のパス、`Cargo.lock`、`src` と `tests` の `use coffee_log...`、fuzz と pbt のクレート名と設定。
  - `backend/coffee_log/wrangler.toml` の `name`、`database_name`、`bucket_name`、`[vars]` の `R2_BUCKET`、`RP_ID`、`ORIGIN`。
  - `backend/coffee_log_admin/wrangler.toml` の `name`、`database_name`、`[vars]` の `APP_ORIGIN` (管理者 Worker は R2 のバインディングを持たない)。
  - `backend/coffee_log/cors.json` のオリジン (例示のホスト名。本番の値はデプロイの前に置き換える)。
  - `frontend/pubspec.yaml` の `name`、`package:coffee_log/...` の import、`frontend/web/manifest.json`、`frontend/web/index.html`、`frontend/lib/l10n/app_en.arb` と `frontend/lib/l10n/app_ja.arb` の `appTitle` と `homeTitle`、`frontend/lib/settings/settings_services.dart` の `exportFileName`、`frontend/ios/Runner/Info.plist` (CFBundleDisplayName と CFBundleName)、`frontend/ios/Runner.xcodeproj/project.pbxproj` (`com.example.coffeeLog`)、`frontend/README.md`。`CoffeeLogApp` のような識別子も改名する。
  - `mise.toml` のタスクの `dir`、`-p coffee_log` と `-p coffee_log_admin` の指定、コメント。
  - `README.md` の Worker 名、D1 のコマンド (`wrangler d1 create coffee-log`)、ホスト名、R2 のバケット名、管理者 Worker の名前。
  - `docs/prd/coffee-log.md` のファイル名と本文、`docs/adr/0001-backend-rust-on-cloudflare-workers.md` の PRD 参照。
- pending と open の issue ファイル (0009、0010、0012、0017、0018、0019、0020) の旧名と旧パスは新名に読み替える。0021 と 0022 は本 issue の実装の時点で closed にあり、更新の対象にしない。
  closed の issue、`CHANGES.md` の過去のエントリ、ADR-0001 から ADR-0009 の本文は当時の記録として変更しない。
  名前と名前空間について ADR-0001、ADR-0004、ADR-0005、ADR-0008 が定めた部分は ADR-0010 が置き換える。
- Cloudflare のリソースは未デプロイのため、新しい名前で作る。旧名の D1 と R2 がアカウントに既にある場合は、どちらの名前も後から変更できないため、新名で作り直してデータを移す。手元のローカルの D1 は `mise run db-migrate` で作り直せる。
- iOS の bundle identifier は未配布のため `com.example.brewBook` に変更する。パスキーの Associated Domains の設定は iOS の実装時に決める (ADR-0004、ADR-0007)。
- 採らなかった案: クレート名だけを変えて Worker 名を残す (画面と URL に現れる名前とコードの名前が分かれる。ADR-0010)、複数の issue に分ける (途中の状態でビルドが通らない。ADR-0010)。

## 完了条件

- `git grep -l -i -E 'coffee[ _-]?log'` の結果が、`CHANGES.md`、`docs/issues/closed/` 以下、`docs/adr/0001-backend-rust-on-cloudflare-workers.md` から `docs/adr/0010-app-name-and-namespace-brewbook.md`、`docs/prd/brewbook.md`、本 issue のファイルだけになる (ADR-0010 と closed の issue は変更前の名前を記録として持ち、PRD は変更の経緯を持ち、本 issue は改名の説明に持つ)。backend/、frontend/、`mise.toml`、`README.md`、`.github/`、pending の issue (0009、0010、0012、0017、0018) と open の issue (0019、0020、0024、0025、0026、0027、0028) には現れない。
- `docs/prd/brewbook.md` の旧名は、変更の記録の 2 行 (2026-09-21 の決定と 2026-09-26 の変更) だけになる。`git grep -n -i -E 'coffee[ _-]?log' -- docs/prd/brewbook.md` が 2 行になる。
- `docs/prd/brewbook.md` が存在し、`docs/prd/coffee-log.md` が存在しない。
- `backend/brew_book`、`backend/brew_book_admin`、`backend/brew_book_core` が存在し、`backend/coffee_log`、`backend/coffee_log_admin`、`backend/coffee_log_core` が存在しない。
- `backend/brew_book/wrangler.toml` の `name`、`database_name`、`bucket_name`、`R2_BUCKET`、`RP_ID`、`ORIGIN` と、`backend/brew_book_admin/wrangler.toml` の `name`、`database_name`、`APP_ORIGIN` が新しい名前になっている。
- `frontend/pubspec.yaml` の `name`、ARB の `appTitle` と `homeTitle`、`frontend/lib/settings/settings_services.dart` の `exportFileName`、`frontend/ios/Runner/Info.plist` の CFBundleDisplayName と CFBundleName、`frontend/ios/Runner.xcodeproj/project.pbxproj` の bundle identifier が上の対応表の値になっている。
- `mise run db-migrate` が成功する。
- `mise run check` が通過する (クレート名の変更後、`backend:test-integration` と frontend の 4 種のテストを含む)。作業の前から失敗している検査がある場合は、同じ失敗だけであることを確認して issue に記録する。
- `mise run dev` で `http://localhost:8787` の画面が表示され、`mise run frontend:test-same-origin` が通過する。
- README のホスト名が `brewbook.<サブドメイン>.workers.dev` と `brewbook-admin.<サブドメイン>.workers.dev` を指し、デプロイのコマンドが `brewbook` と `brewbook-admin` を使う。
- pending と open の issue の旧名と旧パスが新名になっている。
- `CHANGES.md` に `[CHANGE]` のエントリ (「アプリ名と名前空間を brewbook に変更する」) がある。

## 解決方法

アプリ名と名前空間を brewbook / brew_book に変更した (ADR-0010)。

- ディレクトリとファイルの改名 (`git mv`): `backend/coffee_log` → `backend/brew_book`、`backend/coffee_log_admin` → `backend/brew_book_admin`、`backend/coffee_log_core` → `backend/brew_book_core`、`docs/prd/coffee-log.md` → `docs/prd/brewbook.md`。
- 追跡ファイル 152 件の内容 (`git status --porcelain` の 153 行から `CHANGES.md` を除く) の文字列を置換した (`coffee_log` → `brew_book`、`coffee-log` → `brewbook`、`coffeeLog` → `brewBook`、`CoffeeLog` → `BrewBook`、`Coffee Log` → `Brew Book`)。
  対象は `backend/` (100 件)、`frontend/` (41 件)、`docs/` (8 件: ADR-0001 の PRD 参照、`docs/issues/pending/` の 5 件、`docs/issues/0019-bug-dev-server-leaves-workerd-processes.md`、`docs/issues/0020-bug-worker-build-race-in-the-shared-build-directory.md`)、`.gitignore`、`README.md`、`mise.toml` である。
- 記録として残すファイルは置換していない: `CHANGES.md` の過去のエントリ、`docs/issues/closed/` 以下、ADR-0002 から ADR-0010、本 issue のファイル。
- `docs/adr/0001-backend-rust-on-cloudflare-workers.md` は、PRD を指す参照 (9 行目) だけを `docs/prd/brewbook.md` に更新し、本文 (クレート名の記録、26 行目) は変更していない。
- `docs/prd/brewbook.md` はファイル名だけを変えた。本文の現在の名前は先の PRD の更新で既に新しくなっており、旧名は 2026-09-21 と 2026-09-26 の決定の記録 (72 行目と 102 行目) だけである。
- Web の表示名 (`frontend/web/index.html` の `<title>` と `apple-mobile-web-app-title`、`frontend/web/manifest.json` の `name` と `short_name`) は、ADR-0010 の対応表どおりアプリ名の `brewbook` にした (パッケージ名の `brew_book` ではない)。
- pending と open の issue (0009、0010、0012、0017、0018、0019、0020) の旧名と旧パスを新名に読み替えた。
- `CHANGES.md` に `[CHANGE]` のエントリ (「アプリ名と名前空間を brewbook に変更する」) を追加した。

完了条件の検証:

- `git grep -l -i -E 'coffee[ _-]?log'` の結果は、`docs/issues/closed/` 以下、ADR-0001 (26 行目)、ADR-0004、ADR-0005、ADR-0008、ADR-0010、`docs/prd/brewbook.md`、本 issue のファイルだけになった (`CHANGES.md` には旧名は無い)。backend/、frontend/、`mise.toml`、`README.md`、`.github/`、pending の issue、open の issue (0019、0020、0024 から 0030) には現れない。
- `docs/prd/brewbook.md` の旧名は 72 行目と 102 行目の 2 行だけである (決定の記録)。
- `docs/prd/brewbook.md` が存在し、`docs/prd/coffee-log.md` は存在しない。
- `backend/brew_book`、`backend/brew_book_admin`、`backend/brew_book_core` が存在し、`backend/coffee_log`、`backend/coffee_log_admin`、`backend/coffee_log_core` は存在しない。
- `backend/brew_book/wrangler.toml` の `name` は `brewbook`、`database_name` は `brewbook`、`bucket_name` と `R2_BUCKET` は `brewbook-photos`、`RP_ID` は `brewbook.example.workers.dev`、`ORIGIN` は `https://brewbook.example.workers.dev` である。`backend/brew_book_admin/wrangler.toml` の `name` は `brewbook-admin`、`database_name` は `brewbook`、`APP_ORIGIN` は `https://brewbook.example.workers.dev` である。
- `frontend/pubspec.yaml` の `name` は `brew_book`、ARB の `appTitle` と `homeTitle` は `brewbook`、`frontend/lib/settings/settings_services.dart` の `exportFileName` は `brewbook-export.json`、`frontend/ios/Runner/Info.plist` の CFBundleDisplayName は `Brew Book`、CFBundleName は `brew_book`、`frontend/ios/Runner.xcodeproj/project.pbxproj` の bundle identifier は `com.example.brewBook` である。
- `mise run db-migrate` が成功した。
- `mise run check` が通過した。実行の内訳: 改名後の初回の既定の並列実行は `mise run fmt` (`cargo fmt --all --check`) が失敗し (`cargo fmt --all` で修正)、直列実行 (`mise run --jobs 1 check`) の 1 回目と 2 回目はスケールテスト `wrangler_account_delete_removes_the_assumed_scale_of_the_r2_objects_by_cursor` が miniflare の接続断で失敗し (0030)、3 回目の直列実行で通過した (1932 秒)。直列実行を選んだのは、既定の並列実行で起きる Chrome の起動失敗 (0029。0022 の実装時に並列実行で 2 回発生) を避けるためである。CI は既定の並列実行の `mise run check` を使うため、改名後に CI で通過することは未確認である。
- `mise run dev` を起動し、`http://localhost:8787/` が 44 秒で応答することを確認した。HTML は `<title>brewbook</title>` と `flutter_bootstrap.js` を含み、SPA のルート `/register` も 200 (index.html) を返した。`mise run frontend:test-same-origin` は `mise run check` に含まれる同一オリジンの統合テスト (0017) で通過した。
- README のホスト名が `brewbook.<サブドメイン>.workers.dev` と `brewbook-admin.<サブドメイン>.workers.dev` を指し、デプロイのコマンドが `brewbook` と `brewbook-admin` を使う。
- pending と open の issue の旧名と旧パスが新名になっている。
- `CHANGES.md` に `[CHANGE]` のエントリがある。

方針からの乖離: 無し。
補足: 文字列の置換で Rust の識別子名が短くなり、rustfmt の行幅に収まって折り返しが結合されたため、`cargo fmt --all` の整形が 3 ファイルで必要になった (整形のみで、意味は変えていない)。
補足: ディレクトリとファイルの改名 (`git mv`) は、その後の bug issue 0030 の登録のコミット (`4d41195`) に混入してコミットされた (`git mv` が移動をステージ済みにし、その後の git add が他のステージ済みの変更を外さないため)。本 issue の close のコミットには、rename 後のパスに対する内容の変更 (153 ファイル) と本 issue の close を含める。以後のコミットはパスを指定して行う (`git commit -- <パス>`)。
