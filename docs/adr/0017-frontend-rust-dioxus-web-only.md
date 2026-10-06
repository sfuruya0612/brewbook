# ADR-0017: Frontend を Rust (Dioxus) で書き、Web だけを対象にする

Created: 2026-10-02
Model: DeepSeek V4.1 Flash
Status: Accepted

## 背景

ADR-0007 は、Frontend を Flutter で書き、1 つのコードベースから Web と iOS をビルドし、Web を先行して iOS はビルドできる状態を保つと決めた。
iOS の配布形態と Apple Developer Program への加入は未確定のまま残っていた (PRD の未確定論点)。

2026-10-01 に所有者は、iOS 版の対応を要件から落とし、Frontend を Rust に置き換えることを決めた。
デザインはそのままにし、コードだけを置き換える。
技術スタックは Dioxus と Tailwind CSS とし、バンドラは Trunk ではなく Dioxus CLI (`dx`) を使う (2026-10-01 に所有者が確認した)。

次の観測できる事実がこの決定を支える。

- デザインは `docs/design/` に HTML と CSS で存在する。
  `docs/design/tokens.css` は Paper と Night の 2 テーマの CSS 変数を持ち、`docs/design/components/bundle.css` は部品 10 種と画面の CSS クラスを持ち、`docs/design/components/<名前>/preview.html` はブラウザで開けるプレビューである。
  現在の Flutter 実装は、これを `frontend/lib/theme/app_theme.dart` と `frontend/lib/theme/tokens.dart` に写して `ThemeData` に落とし込んでいる。
- Frontend の実装は `frontend/lib` の約 14,000 行と `frontend/test` の約 7,000 行である (2026-10-01 に `wc -l` で計測)。
  API は Backend が FR-19 の正常系を除いて CI で全経路をテスト済みで、画面の仕様は `docs/design/` に揃っている。
- 写真の選択と変換 (FR-10)、パスキー (FR-1 から FR-3)、ファイルの保存 (FR-14) は、Web と iOS で実装が分かれる部分として `*_stub.dart` と `*_web.dart` の 4 組に抽象化されている (ADR-0007 の結果)。
  iOS を対象から落とすと、この抽象化は不要になる。
- ビルド成果物は Cloudflare Workers の Static Assets として Backend と同じ Worker から配信する (ADR-0005)。
  Web のレンダラは CanvasKit で、初回のロードのサイズは問題があれば skwasm へ変更するとしていた (ADR-0007 の結果)。
- テストは 5 種 (E2E、PBT、Fuzzing、形式手法、単体) のうち Fuzzing の実行以外を CI で実行し、Fuzzing は CI では対象の型検査だけを行う (ADR-0013)。
  パスキーを伴う E2E は ChromeDriver の WebAuthn の拡張コマンドで仮想認証器を付け、chromedriver を `mise.toml` で固定している。
- ローカルの開発は「ビルドしてから `wrangler dev`」で、Frontend の開発サーバーは別オリジンになるため使わない (ADR-0005)。

## 決定

- Frontend は Rust で書き、Web だけを対象にする。
  iOS 向けのビルドと配布は要件から落とし、将来も含めて対象にしない (所有者の決定)。
- UI は Dioxus 0.7 系で組む。
  ビルドは Dioxus CLI (`dx`) を使い、Trunk は使わない。
- スタイルは Tailwind CSS を使い、`docs/design/tokens.json` を原本とし、その CSS 変数 (`docs/design/tokens.css`) を Tailwind のテーマに取り込む。
  色、余白、角丸、書体、部品の値は `docs/design/` を原本とし、実装は原本に合わせる。
- 画面の経路は 1 か所の台帳に置き、PRD の成功指標 (画面数) を数えられるようにする (ADR-0007 の決定を維持する)。
- グラフ (FR-18) は Frontend で描画し、集計は Backend の API が行う (ADR-0007 の決定を維持する)。
  描画の方法は実装の issue で決める。
- 写真の JPEG 変換と縮小 (FR-10)、パスキー (FR-1 から FR-3)、ファイルの保存 (FR-14) はブラウザの API を Rust から直接使う。
  Web と iOS のプラットフォームごとの抽象化は置かない。
- UI の文言は型付きのキーと日本語と英語の表で持ち、画面のコードに表示する文字列を直接書かない。
  日本語と英語の対応は維持する (FR-16)。
- テストは 5 種の枠組みを維持し、Frontend は native の `cargo test` (単体と PBT)、`wasm-bindgen-test` (ブラウザの API に依存する検証)、E2E (実 Worker と仮想認証器) を持つ。
- 依存クレートの追加は、理由を ADR または issue に記録した上で行う (ADR-0001 の規約を維持する)。
- ADR-0007 を置き換える。
  ADR-0007 の Status を「Superseded by ADR-0017 (2026-10-02)」に変え、本文の冒頭に置き換えの注記を足す (同じ変更で行う)。

## 検討した選択肢

| 選択肢 | 採用しない理由 |
| --- | --- |
| Flutter を続け、iOS だけを落とす | 所有者が Frontend も Rust にすることを求めた。iOS を落とすと 4 組の抽象化は不要になり、CanvasKit の初回ロードの制約だけが残る |
| Dioxus + Trunk | 所有者が `dx` を選んだ。`dx` はプロジェクトのルートの `tailwind.css` を検出して Tailwind CLI をダウンロードして起動し、`dx serve --hotpatch` の開発体験を持つ (Dioxus 0.7 のドキュメント: https://dioxuslabs.com/learn/0.7/essentials/ui/styling と https://dioxuslabs.com/learn/0.7/essentials/ui/hotreload)。Trunk は汎用の wasm バンドラで、この 2 つを使うには別途の配線が要る |
| Leptos | 所有者が Dioxus を選んだ。Leptos は SSR と islands に強みがあるが、Web だけの SPA では選ぶ理由が無い |
| Yew | 所有者が Dioxus を選んだ (技術的な比較は行っていない)。採用しない技術的な理由は特定していない |
| 素の wasm-bindgen と web-sys だけで書く | 部品 10 種と 18 経路を持つアプリの状態管理と DOM の差分更新を自前で持つことになる |
| TypeScript (React など) に置き換える | 所有者が Rust への統一を求めた。Backend とテストとツールチェーンを 1 つの言語に揃えられる |
| iOS の対応を続ける | 所有者が要件から落とした。Apple Developer Program への加入も配布形態も未確定のままである |

## 結果

- `frontend/` は Flutter のパッケージから Rust のクレートに置き換わる。
  Frontend のクレート名は `brew_book_frontend` とする (`brew_book` は利用者向けの Worker が使っているため。ADR-0010 の名前空間に 1 つ追加する)。
  `frontend/ios/`、`frontend/lib/`、`frontend/test/`、`frontend/integration_test/`、`frontend/test_driver/`、`frontend/web/`、`frontend/README.md`、`frontend/.gitignore`、`pubspec.yaml`、`pubspec.lock`、`analysis_options.yaml`、`l10n.yaml`、`.metadata` は使わなくなる (追跡外の `flutter_0*.log` と `.flutter-plugins-dependencies` はあれば削除する)。
  `frontend/web/` の `index.html` の `<title>` と表示名 (`apple-mobile-web-app-title`) と `manifest.json` の `name`、`short_name`、favicon、icons、テーマ色は Dioxus の `index.html` と manifest に引き継ぐ。
  `apple-touch-icon-180.png` は iOS を対象から落とすため引き継がない。
- Flutter のツール (`mise.toml` の `flutter` と `frontend:*` の 8 タスク) は、Dioxus のツールとタスクに置き換わる。
  移行の間は両方を置き、配信の差し替えと Flutter の削除は最後の issue で行う。
- Flutter の成果物のパスを直接参照する箇所は、Dioxus の成果物に合わせて更新する。
  対象は `backend/brew_book/wrangler.toml` の `[assets]` の `directory`、`backend/brew_book/tests/support/mod.rs` と `backend/brew_book_admin/tests/support/mod.rs` のアセットのパス、`backend/brew_book/tests/wrangler_same_origin_api.rs` と `backend/brew_book/tests/wrangler_same_origin_e2e.rs` の Flutter の成果物名の検証である。
- CI から Flutter を外す。
  Chrome for Testing と chromedriver は `wasm-bindgen-test` と E2E のために残す。
- README.md と `docs/design/` の実装への対応を更新する。
  原本の値 (`tokens.json` の値、`tokens.css`、プレビュー、スクリーンショット) は変えず、実装への対応 (`docs/design/README.md`、`docs/design/components/*/README.md`、`docs/design/INDEX.md`、`tokens.json` の実装の注記) を Dioxus と Tailwind に変える。
- Web のレンダラの選択 (CanvasKit) は無くなり、CanvasKit 由来の初回のロードの懸念は消える。
  新しいバンドルのサイズは未計測であるため、ビルドの成果物のサイズ (gzip 後) を計測して issue に記録する (移行の issue 0037)。
  ADR-0007 の結果の「問題があれば skwasm へ変更する」は取り消す。
- 負荷試験 (ADR-0012) の対象の静的アセットは `index.html` と JavaScript のローダーと WebAssembly に変わる。
  k6 のシナリオと README を更新する。
- FR-16 の翻訳ファイルと静的検査、概要、成功指標の測定方法、スコープ、性能、制約と前提、未確定論点は本 ADR に合わせて PRD を改訂する (同じ変更で 2026-10-02 に改訂する)。
- iOS と Flutter を前提にした記述は、本 ADR の決定に合わせて不要になったものを消す。
  対象は、ADR-0004 の iOS の記述 (パスキーのネイティブ連携と選択肢の表)、ADR-0005 の Flutter のルーティングとビルドと iOS 向けの配信 (Cookie と Bearer トークンを含む)、ADR-0008 の管理者画面の Flutter の記述、ADR-0009 の Flutter の版の固定と Flutter を前提にした記述 (背景とタスクの規約)、ADR-0010 の Flutter と iOS の名前と iOS の Associated Domains の記述 (名前空間への `brew_book_frontend` の追加を含む)、ADR-0012 の Flutter の成果物、ADR-0013 の Flutter の PBT とウィジェットテストと統合テストと E2E の方式、ADR-0014 の Flutter と iOS の記述 (`google_fonts`、`FontLoader`、アセットへの同梱の再検討) である。
  ADR-0003 は iOS と Flutter の記述が無いため改訂しない。
  改訂は、移行の issue 0039 (ADR-0014)、0040 (ADR-0004)、0044 (ADR-0013)、0045 (ADR-0005、ADR-0008、ADR-0009、ADR-0010、ADR-0012) で行う。
- Dioxus 0.7.10 が 2026-07-30 (UTC) に公開され、2026-10-01 時点で最新の安定版である (GitHub のリリース: https://github.com/DioxusLabs/dioxus/releases)。
  0.8.0-alpha.1 が 2026-07-31 (UTC) に公開されており、0.8 系が並行して進んでいる。
  版は `mise.toml` と `Cargo.lock` で固定し、更新は別の issue で行う。
- 2026-09-10 に Dioxus Labs が Cognition へ参画し、Dioxus、Blitz、Taffy、Subsecond の開発を続けると発表した (https://dioxuslabs.com/blog/joining-cognition)。
  開発の継続のリスクとして記録する。
