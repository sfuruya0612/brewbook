# ADR-0013: テストを E2E、PBT、Fuzzing、形式手法、単体の 5 種で網羅する

Created: 2026-09-26
Model: DeepSeek V4.1 Flash
Status: Accepted
改訂: 2026-10-02 (issue 0044。Frontend を Dioxus の Web だけにする決定 (ADR-0017) に合わせて、Flutter のテストの記述を Rust のテストに改めた)

## 背景

PRD の「制約と前提」は、Backend を Rust のテスト規約 (単体テストは `tests/test_<module>.rs`、PBT は `pbt/tests/prop_<module>.rs`、Fuzzing) に従わせ、Frontend のテストを持つと定める。
Frontend のテストは、Frontend を Dioxus の Web だけにする決定 (ADR-0017、2026-10-01) に合わせて、Rust の単体テスト、PBT、ブラウザのテスト (`wasm-bindgen-test`)、E2E になった (0044)。
所有者の共通規約も、PBT を単体テストより先に検討し、単体テストは PBT で実現できないものだけを書くことを求める。

2026-09-26 時点のテストの種類ごとの現状は次の通り。

| 種別 | 現状 |
| --- | --- |
| 単体 | Rust は各クレートの `tests/` にある (Frontend は `frontend/tests/`) |
| E2E | 実バックエンドと仮想認証器で登録、ログイン、抽出の保存を検証する統合テストがある (0017)。Frontend は Dioxus のアプリを WebDriver で操作する (0044) |
| PBT | `backend/pbt/tests/prop_*.rs` に 14 ファイルがある。Frontend は `frontend/pbt/tests/prop_*.rs` にある (0038) |
| Fuzzing | fuzz のクレートの `fuzz_targets/parse.rs` の 1 ターゲットが CBOR、COSE、base64url、WebAuthn の検証を対象にする。他の入力のパーサは対象外である |
| 形式手法 | 無い |

所有者が 2026-09-26 に、テストを E2E、PBT、Fuzzing、形式手法、単体の 5 種で網羅することを決めた。

## 決定

5 種の役割を次のように定める。

- **単体**: 意図的なエラーパスと境界値を確認する。PBT で実現できるものは PBT に置き、単体テストに書かない (所有者の共通規約)。
- **PBT**: 型に基づいて入力を生成し、性質 (往復、不変条件、境界の一般化) を検証する。
  Rust は `backend/pbt/tests/prop_<module>.rs` と `frontend/pbt/tests/prop_<module>.rs` に置く。
- **Fuzzing**: 任意入力に対するパニック安全性を確認する。
  Rust の入力のパーサを対象とし、実行は nightly を要するため手元で行う (0004 の決定を維持する)。
  CI では対象の型検査だけを行う。
- **形式手法**: 状態遷移が本質になる対象を TLA+ の仕様として書き、TLC のモデル検査で不変条件を検証する。
  対象は WebAuthn のチャレンジ (1 回だけ消費される。FR-1、FR-2)、登録用トークン (1 回だけ使用される。FR-1)、セッション (ログアウトと期限切れの後は通さない。FR-4)、パスキー (最後の 1 つは削除できない。FR-3) の状態遷移とする。
  仕様は `formal/` に置き、CI (`mise run check`) で実行する。
- **E2E**: 実際の Worker とブラウザを組み合わせ、主要なユースケースを最初から最後まで検証する。
  認証を伴う経路は ChromeDriver の WebAuthn の拡張コマンド (`WebAuthn.addVirtualAuthenticator`) で仮想認証器を付ける (0044。`flutter drive` をやめたため)。
  現状の経路 (登録、ログイン、抽出の保存) を維持し、経路を増やす必要が生じたときに同じ方式で足す。

道具は次のように選ぶ。

- Frontend の PBT は `proptest` を使う (Backend と同じ。ADR-0017 の移行で、Flutter の `kiri_check` (Dart 3 対応。2026-09-26 時点の版は 1.3.1) は対象から外れた)。
- 形式手法は TLA+ と TLC を使い、JDK は Temurin 25 (LTS) を `mise.toml` で固定する。
  `tla2tools.jar` は版 (v1.7.4。2026-09-26 時点の最新リリース) と SHA-256 (`936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88`。同日に取得して確認) を記録して取得し、リポジトリに含めない。
  mise のレジストリに tla2tools は無いため、`mise.toml` の `[tools]` では取得できず、タスクで取得する。

5 種のうち、Fuzzing の実行以外は `mise run check` で実行する。
Fuzzing の実行だけは nightly を要するため手元で行い、CI では対象の型検査とする。

## 検討した選択肢

| 選択肢 | 採用しない理由 |
| --- | --- |
| 既存の種別 (単体、PBT、Fuzzing) のままにする | チャレンジの再利用やセッションの失効のような状態遷移の誤りを、テストの入力の組合せでは網羅しきれない |
| 形式手法に Kani (Rust のモデル検査) を使う | 対象の状態遷移はデータベースの行をまたぐため、実装の詳細に依存する。設計の水準で書ける TLA+ のほうが対象に合う。所有者が TLA+ を選んだ |
| 形式手法に Alloy を使う | 対象は表現できるが、所有者が TLA+ を選んだ。既存の資料と例が多いことも TLA+ を選んだ理由である |
| Flutter の PBT に `kiri_check` を使う | Frontend を Dioxus の Web だけにする決定 (ADR-0017) で、Flutter の実装ごと対象から外れた。Rust の PBT は Backend と同じ `proptest` を使う (0044) |
| 形式手法の仕様を手元での実行だけにする | 変更のたびに CI で検査できないと、仕様と実装が離れていく。TLC の実行は短く、CI に含められる |

## 結果

- 不足していた形式手法 (TLA+)、Frontend の PBT、Fuzzing の対象の拡大は、それぞれ issue 0026、0027、0028 で行う。
  Frontend の PBT は、Frontend を Dioxus の Web だけにする決定 (ADR-0017) に合わせて、issue 0038 以降が Rust の `frontend/pbt/` に移した (Flutter の PBT の issue 0027 は、Flutter の削除 (0045) の後に扱いを決める)。
- E2E と単体テストは現状のテストを維持する。E2E のブラウザの操作は、Flutter の `flutter drive` から WebDriver (chromedriver) に変わった (0044)。
- 形式手法の仕様と実装の対応 (どの不変条件がどの要求に対応するか) は `formal/README.md` に記録する。
- テストの種別を増やすときは、本 ADR を改訂する。
