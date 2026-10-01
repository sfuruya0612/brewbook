# Dioxus の Frontend 基盤を作る

Created: 2026-10-02
Model: deepseek-v4p1-flash
対応 ADR: ADR-0017、ADR-0005 (docs/adr/0005-same-origin-deployment.md)、ADR-0013 (docs/adr/0013-five-test-layers.md)
関連 PRD: FR-16、制約と前提 (テスト)、非機能要求 運用
依存: 0037

## 背景

本 issue は、2026-10-01 の所有者の決定 (ADR-0017) から起票した。

Flutter の基盤は次のファイルにある。
`frontend/lib/router/app_router.dart` は 18 経路の台帳 (`AppRoutes`) を持ち、経路の名前を経路のパターンと同じにして、画面数の成功指標を数えられるようにしている。
`frontend/lib/api/api_client.dart` は同一オリジンの `/api` を呼び、エラーの JSON を `ApiError` に変換し、401 で `onUnauthorized` を呼び、応答を取得できない失敗を `NetworkError` にする。
`frontend/lib/records/values.dart` と `frontend/lib/records/stats_period.dart` は値の変換と統計の期間の組み立てを持つ。
`frontend/lib/l10n/app_ja.arb` と `app_en.arb` は 227 キーの翻訳を持つ (複数形と選択は無い。2026-10-02 に確認)。
`frontend/test/l10n_check_test.dart` は UI のコードに文言を直書きしていないことと、日本語の 8 つの訳語を検査する (FR-16)。

## 目的

Dioxus のアプリの基盤 (クレートの構成、経路の台帳、API クライアント、値の変換、翻訳、起動時のセッション確認) が動き、画面の実装を載せられる状態にする。

## 設計判断

- クレートは 0037 の `frontend/` のワークスペースの `brew_book_frontend` を使う。
  Dioxus の依存は `dioxus` (web の feature) と `dioxus-router` に限定し、状態管理のライブラリは追加しない (ADR-0007 の「依存を最小にする」方針を ADR-0017 が引き継ぐ)。
  HTTP は `gloo-net` と `web-sys` の `fetch` を比較し、依存の少ない方を選ぶ (実装の最初に決めて記録する)。
- 経路の台帳は `frontend/src/router.rs` の 1 か所に置き、経路の名前は経路のパターンと同じにする。
  未知の経路はホームへ戻す (Dioxus のルーターの fallback として定める。Flutter は go_router の既定のエラー画面で、同じ挙動ではないため、意図的な変更として記録する)。
- 起動時のセッション確認は `GET /api/passkeys` の 401 で判定する (Flutter と同じ。専用のセッション確認 API は設けない)。
- 翻訳は型付きのキーの列挙と日本語と英語の定数の表で持つ。
  `t(Key::SaveButton)` の形にして、表示する文字列を直接書けないようにする。
  ARB と ICU の複数形は使わない (227 キーに複数形と選択は無い)。
  キーの列挙と 2 つの言語の表の対応 (全件) をテストで照合する。
- 文言の直書きの静的検査は native のテスト (`frontend/tests/test_i18n.rs`) として書き、`frontend/src/ui/` と `frontend/src/screens/` の `.rs` の `rsx!` で、次の位置の文字列リテラルを検出する。対象は (1) 要素の本文、(2) `placeholder`、`title`、`aria-label`、`alt` の属性の値、(3) デザインの部品 10 種の `label` と `title` の prop の値とする。
  対象外は、上記以外の属性の値 (Tailwind の `class`、`type`、`id`、`name`、`for`、`href`、`src`、`rel`、`role`、`value`、`kind`、`accept`、`autocomplete`、`inputmode`、`pattern`、`min`、`max`、`step`、`maxlength`、`method`、`action`、`target`)、`t(` の呼び出し、ログ、テストコード、翻訳の表、URL とする (Flutter の `l10n_check_test.dart` の検査を Rust に写す。FR-16)。
- 値の変換と統計の期間は Dioxus に依存しない純粋なモジュールとして写し、native の単体テストと `proptest` の PBT で守る (ADR-0013)。
  これが移行の品質の中心になる。
- 記録の依存 (API クライアント、時計、写真の選択と変換) を束ねた型を用意し、画面に配る (Flutter の `RecordServices` と同じ。0041 が使う)。
- API の入力と応答の対応 (要求の JSON と応答の JSON の往復) の PBT を native のテストとして持つ (ADR-0013)。
- 翻訳のキーの列挙は、日本語の 8 つの訳語 (生産者、生産国、地域、精製方法、品種、焙煎度、焙煎日、フレーバーノート) を値として持ち、テストで照合する (FR-16)。
- 採らない案: `dioxus-i18n` クレートを使う (227 キーの単純な表には過剰で、型で直書きを防げない)、`fluent` を使う (複数形が無いため不要)、`reqwest` を wasm で使う (依存が大きい)、状態管理のクレートを入れる (ADR-0007 の決定を維持する)。

## 完了条件

- `mise run dioxus:build` が基盤のアプリをビルドし、`mise run dioxus:lint` が警告なしで成功する。
- 経路の台帳に Flutter と同じ 18 経路がある (native のテストで経路の一覧を照合する)。
- API クライアントが `/api` の下を呼び、エラーの JSON を共通の型に変換し、401 でログイン画面へ遷移させ、ネットワークエラーで再試行を促す (native のテストで確認する)。
- 翻訳が日本語と英語であり、端末またはブラウザの言語が日本語なら日本語、それ以外なら英語になる (native のテストで確認する)。
- 日本語の翻訳ファイルに 8 つの訳語が値として存在する (native のテストで確認する)。
- `frontend/src/ui/` と `frontend/src/screens/` の UI コードの `rsx!` に、要素の本文と表示に使う属性の値に文字列リテラルの直書きが無い (native のテストで確認する)。
- 値の変換と統計の期間の単体テストと PBT がある。
- API の入力と応答の対応の PBT がある。
- 記録の依存を束ねた型があり、画面に配れる (native のテストで確認する)。
- `mise run check` が通過する。

## 関連

- 0037 がツールチェーンとタスクを作る。
- 0039 がデザインシステムを、0040 以降が画面を作る。

---
