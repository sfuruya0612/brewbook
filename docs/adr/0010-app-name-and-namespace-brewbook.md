# ADR-0010: アプリ名と名前空間を brewbook に変更する

Created: 2026-09-26
Model: DeepSeek V4.1 Flash
Status: Accepted

## 背景

PRD は 2026-09-21 に、アプリ名を coffee-log、Flutter のパッケージ名を coffee_log、Rust のクレートを coffee_log (利用者向けの Worker)、coffee_log_admin (管理者 Worker)、coffee_log_core (共有ライブラリ) とすることを決めた (ADR-0001)。

2026-09-26 に所有者が、coffee_log の log はログ (logging) の名前空間と紛らわしく、記録のアプリの名前として分かりにくいと判断した。

名前はコードの中だけでなく、Worker 名 (coffee-log、coffee-log-admin)、D1 データベース名 (coffee-log)、R2 バケット名 (coffee-log-photos)、Flutter のパッケージ名と import、iOS の表示名と bundle identifier (com.example.coffeeLog)、PRD のファイル名 (docs/pdr/coffee-log.md) にも現れる。

本番へのデプロイはまだ行われていない (0017 と 0018 が pending である)。
そのため、Cloudflare のリソース名を変更しても、本番のデータの移行は要らない。

## 決定

アプリ名を **brewbook** とする。
記録 (book) の名前空間とし、log (ログ) の語を使わない。

名前は次の対応で統一する。

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
| Web の表示名 (`index.html` の `<title>` と `apple-mobile-web-app-title`、`manifest.json` の `name` と `short_name`) | coffee_log | brewbook |
| Web と README の説明 (`index.html` と `manifest.json` の `description`、`frontend/README.md` の 3 行目、`frontend/pubspec.yaml` の 1 行目と 4 行目) | coffee-log の Frontend (ADR-0007) | brewbook の Frontend (ADR-0007) |
| `frontend/README.md` の見出し | coffee_log | brew_book |
| 画面のアプリ名 (ARB の appTitle と homeTitle) | coffee-log | brewbook |
| エクスポートのファイル名 | coffee-log-export.json | brewbook-export.json |
| iOS の表示名 (CFBundleDisplayName) | Coffee Log | Brew Book |
| iOS の CFBundleName | coffee_log | brew_book |
| iOS の bundle identifier | com.example.coffeeLog | com.example.brewBook |
| PRD | docs/pdr/coffee-log.md | docs/prd/brewbook.md |

- 変更は 1 つの issue で一括して行う。
  クレート名、Worker 名、D1 の名前、Flutter のパッケージ名は同じ文字列を参照し合うため、一部だけを変えるとビルドとテストが通らない。
  PRD のディレクトリ名の誤り (`docs/pdr`) は別に issue 0022 で `docs/prd` に直す。本 ADR の表の「変更後」の `docs/prd/brewbook.md` は、0022 の後に行う 0023 の完了時の状態である (ファイル名は 0023 が変える)。
- 利用者向けの Worker のホスト名は `brewbook.<アカウントのサブドメイン>.workers.dev`、管理者 Worker は `brewbook-admin.<アカウントのサブドメイン>.workers.dev` になる。
  パスキーの RP ID と Origin、R2 の CORS のオリジン、管理者 Worker の APP_ORIGIN も新しいホスト名に合わせる (ADR-0004、ADR-0005、ADR-0008)。
- Cloudflare のリソースは新しい名前で作る。
  旧名の D1 と R2 がアカウントに既にある場合は、どちらの名前も後から変更できないため、新名で作り直してデータを移す。
  手元のローカルの D1 は `mise run db-migrate` で作り直せる。
- iOS の bundle identifier (`com.example.coffeeLog`) は未配布のため変更する。
  パスキーの Associated Domains の設定は iOS の実装時に決める (ADR-0004、ADR-0007)。
- open と pending の issue (0009、0010、0012、0017、0018、0019、0020) の旧名と旧パスは新名に更新する。
  実装が残っている issue が古い名前とパスを参照すると、pending の再開時に実装できない。
- 過去の記録 (closed の issue、CHANGES.md の過去のエントリ、ADR-0001 から ADR-0009) は当時の記録として変更しない。
  ただし PRD を指す参照 (README.md の 5 行目と ADR-0001 の 9 行目) だけは、issue 0022 と 0023 が新しいパスに更新する (0022 がディレクトリ名を、0023 がファイル名を変える)。
  名前と名前空間について ADR-0001、ADR-0004、ADR-0005、ADR-0008 が定めた部分は ADR-0010 が置き換える。
  それ以外の決定 (言語、ワークスペース構成、配信方式、Access の保護) は変えない。

## 検討した選択肢

| 選択肢 | 採用しない理由 |
| --- | --- |
| coffee_log のまま、ディレクトリの名前だけを変える | log と logging の紛らわしさが残る。所有者の指摘の対象は名前そのものである |
| coffee-note にする | 記録を表す名前ではあるが、所有者が brewbook を選んだ (2026-09-26) |
| リポジトリのディレクトリ名に合わせて coffee にする | 一般的な名前すぎて、workers.dev のホスト名と D1 の名前が他の利用者と衝突しやすい。抽出 (brew) の記録という役割も表さない |
| 名前の変更を複数の issue に分ける | クレート名、Worker 名、D1 の名前、Flutter のパッケージ名が同一の文字列を共有するため、途中の状態ではビルドとテストが通らない |
| Worker 名だけを残してコードの名前を変える | 画面と URL に現れる名前とコードの名前が分かれ、探しにくくなる |

## 結果

- 名前空間は利用者向け (brew_book)、管理者 (brew_book_admin)、共有 (brew_book_core) に分かれ、logging と区別できる。
- 名前の変更は issue 0023 で行う。PRD のディレクトリ名は、その前に issue 0022 が直す。
- PRD の「制約と前提」と「関連資料」、README のデプロイ手順、`mise.toml` のタスクも新しい名前になる。
- 本 ADR は名前と名前空間に関する決定だけを扱う。
  テストの種別 (ADR-0013)、負荷試験 (ADR-0012)、CI の action の固定 (ADR-0011) は別の ADR で扱う。
