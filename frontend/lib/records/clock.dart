/// 端末の時計とタイムゾーンの依存 (FR-9、FR-11、FR-18)。
///
/// 統計の画面は、端末のタイムゾーンでの当月の期間を組み立て、端末のローカル時刻から
/// UTC を引いた分数を API に渡す。テストが端末の時計とタイムゾーンを差し替えられるよう、
/// この型で画面へ配る (ADR-0007)。
library;

/// 実行環境の時計とタイムゾーン。
class DeviceClock {
  const DeviceClock();

  /// 端末のタイムゾーンでの現在の日時。
  DateTime now() => DateTime.now();

  /// 端末のローカル時刻から UTC を引いた分数 (FR-18)。日本標準時は +540。
  int utcOffsetMinutes() => DateTime.now().timeZoneOffset.inMinutes;
}
