/// テスト用の固定の時計とタイムゾーン (FR-18)。
library;

import 'package:brew_book/records/clock.dart';

/// 決まった日時と UTC オフセットを返す端末の時計。
///
/// 統計の期間は端末のタイムゾーンでの当月に依存し、UTC オフセットは端末の値を使うため、
/// テストはこの実装で期間とオフセットを固定する (+540 は日本標準時)。
class FixedDeviceClock implements DeviceClock {
  FixedDeviceClock({required this.value, required this.offsetMinutes});

  /// 端末のタイムゾーンでの現在の日時として返す値。
  final DateTime value;

  /// 端末のローカル時刻から UTC を引いた分数。
  final int offsetMinutes;

  @override
  DateTime now() => value;

  @override
  int utcOffsetMinutes() => offsetMinutes;
}
