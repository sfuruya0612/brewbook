# Flutter の値の変換と統計の期間と入力の形に PBT を追加する

Created: 2026-09-26
Model: deepseek-v4p1-flash
対応 ADR: ADR-0013 (docs/adr/0013-five-test-layers.md)
関連 PRD: 制約と前提 (テスト)
依存: 0023

本文のパッケージ名は、依存の 0023 の完了後の名前 (`brew_book`) で書く。

## 背景

グローバル規約と PRD の「制約と前提」は、PBT を単体テストより先に検討し、単体テストは PBT で実現できないものだけを書くことを定める。
Rust 側は `backend/pbt/tests/` に 14 の `prop_*.rs` があるが、Flutter 側は `frontend/test/` の単体テストとウィジェットテストだけで、PBT が無い。
ADR-0013 で、Flutter にも PBT を導入することを決めた。

PBT の対象になる純粋なロジックは次のとおり (2026-09-26 に確認)。

- `frontend/lib/records/values.dart` の `formatDay` と `parseDay`、`formatTime` と `parseTime`、`parseCount`、`parseDecimal`、`formatNumber`。
  日付は `YYYY-MM-DD`、時刻は `HH:MM`、数は 0 以上で小数第 1 位までを扱う。
- `frontend/lib/records/stats_period.dart` の `statsPeriodFor`、`statsDayCount`。
  期間の種別 (当月、3 か月、6 か月、12 か月、全期間、任意) を API の開始日、終了日、粒度に変換する。任意の期間は 62 日以下なら日別、それ以外は月別にする。
- `frontend/lib/api/record_inputs.dart` の `toJson` と `frontend/lib/api/models.dart` の `fromJson`。
  入力の型は API の本文の形を作り、応答の型は API の応答の形を読む。重なる項目の名前と型は一致しなければならない。

現在の単体テストは `frontend/test/values_test.dart` などが代表的な境界値を確認している。PBT は入力の組合せ全体に性質を広げる。

## 目的

Flutter の値の変換、統計の期間の組み立て、API の入力と応答の形の対応に PBT を追加し、型に基づく入力の生成で性質を検証する。

## 設計判断

- PBT のライブラリは `kiri_check` を使う。`frontend/pubspec.yaml` の `dev_dependencies` に `kiri_check: ^1.3.1` を追加する。
  - `kiri_check` は Dart 3 に対応し、`package:test` と同じ `property` と `forAll` で書ける。2026-09-26 時点の版は 1.3.1 で、pub.dev の 30 日間のダウンロードは約 1,500 である。
  - 採らなかった案: `glados` (最新版が Dart 2 までで、Dart 3.13 の環境で使えない)、`property_testing` (0.3.2+1。Dart 3 に対応するが、30 日間のダウンロードが 19)、`ribs_check` (1.0.0。同 24)。
- テストは `frontend/test/property/` に置き、ファイル名を `<module>_property_test.dart` とする (`values_property_test.dart`、`stats_period_property_test.dart`、`api_json_property_test.dart`)。
  `flutter test` は `test/` 以下の `*_test.dart` を自動で検出するため、CI の `frontend:test` にそのまま含まれる。
- 生成する入力の定義域は、コードの仕様に合わせて性質ごとに定める。日付は年 0000 から 9999 の実在する日付、時刻は時 0 から 23 と分 0 から 59、整数は 0 以上 2^53 以下 (double を経由した往復が正確な範囲)、小数は 0 以上 10^21 未満で小数第 1 位までとする (5 桁の年、24 時以上、10^21 以上は `formatDay`、`parseTime`、`formatNumber` の仕様の外であり、`parseDay` と `parseDecimal` の形式の検査に一致しない)。
- 反例の再現は、kiri_check が出力するシード値を使う。シードを指定する方法 (テストの引数、環境変数など) を確認し、テストのコメントに書く。シードを指定できない場合は、固定の `Random` を使う生成器をテストで組み立てる。
- 採らなかった案: ウィジェットテストに PBT を導入する (画面の状態とタイミングに依存し、性質を安定して書けない。対象は純粋なロジックに限る)、Rust 側と同じ `proptest` 相当の仕組みを自作する (ライブラリの採用と比べて得るものが無い)。

## 完了条件

- `frontend/pubspec.yaml` の `dev_dependencies` に `kiri_check` がある。
- `frontend/test/property/values_property_test.dart` に、次の性質がある。年 0000 から 9999 の実在する日付について `parseDay(formatDay(day))` が元の日付と等しい。実在しない日付 (2 月 30 日など) と形式の違反は `parseDay` が null を返す。時 0 から 23 と分 0 から 59 について `parseTime(formatTime(hour, minute))` が元の時と分と等しい。0 以上 2^53 以下の整数について `parseCount(formatNumber(value))` が元の値と等しい。0 以上 10^21 未満で小数第 1 位までの小数について `parseDecimal(formatNumber(value))` が元の値と等しい。
- `frontend/test/property/stats_period_property_test.dart` に、次の性質がある。`statsPeriodFor` の当月の開始日が月初で終了日が当日である。3 か月、6 か月、12 か月の開始日が年をまたぐ場合を含めて正しい。任意の期間は日数が 62 日以下なら日別、63 日以上なら月別になる。`statsDayCount` が両端を含む日数になり、終了日が開始日より前のときは正の値にならない。
- `frontend/test/property/api_json_property_test.dart` に、次の性質がある。店、商品、購入、抽出の入力の `toJson` が作る JSON に、応答に必要な `id` と日時と入れ子のオブジェクト (購入の `product` と `shop`、抽出の `purchase` とその中の `product` と `shop`) を足して `fromJson` で読むと、重なる項目の値が入力と一致する。
- `mise run frontend:test` で上の 3 ファイルが実行され、通過する。
- 反例が出たときの再現手順 (シードの指定方法) がテストのコメントに書かれている。
- `mise run check` が通過する。作業の前から失敗している検査がある場合は、同じ失敗だけであることを確認して issue に記録する。
- `CHANGES.md` の `### misc` に `[ADD]` のエントリがある。
