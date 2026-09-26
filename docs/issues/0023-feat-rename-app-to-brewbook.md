# アプリ名と名前空間を brewbook に変更する

Created: 2026-09-26
Model: deepseek-v4p1-flash
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
