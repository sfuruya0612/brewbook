/// 統計と評価の推移の API (FR-18) の呼び出し。
///
/// 集計は Backend の集計クエリが行い、このクラスは期間と粒度と UTC オフセットを渡して
/// 集計結果を受け取るだけにする (ADR-0007)。期間の端は端末のタイムゾーンでの日付
/// (`YYYY-MM-DD`) とし、抽出の API だけが UTC オフセット (分) を受け取る。
library;

import 'api_client.dart';
import 'models.dart';

/// 集計の粒度 (FR-18)。
enum StatsGranularity {
  /// 日別 (区間のキーは `YYYY-MM-DD`)。
  day('day'),

  /// 月別 (区間のキーは `YYYY-MM`)。
  month('month');

  const StatsGranularity(this.apiValue);

  /// API のクエリパラメータの値。
  final String apiValue;
}

/// 統計の API を [ApiClient] で呼ぶ。
class StatsApi {
  StatsApi(this._api);

  final ApiClient _api;

  /// 抽出回数と豆の消費量を引く (FR-18)。
  ///
  /// [start] と [end] は端末のタイムゾーンでの日付。全期間では両方を省略する。
  /// [utcOffsetMinutes] は端末のローカル時刻から UTC を引いた分数 (日本標準時は +540)。
  Future<List<BrewPeriod>> brews({
    String? start,
    String? end,
    required StatsGranularity granularity,
    required int utcOffsetMinutes,
  }) async {
    final json = await _api.getJson(
      _statsPath(
        '/stats/brews',
        start: start,
        end: end,
        granularity: granularity,
        utcOffsetMinutes: utcOffsetMinutes,
      ),
    );
    return _objects(json, 'brews').map(BrewPeriod.fromJson).toList();
  }

  /// 購入金額と重量を引く (FR-18)。
  ///
  /// 購入日はタイムゾーンを持たないため、UTC オフセットは渡さない。
  Future<List<PurchasePeriod>> purchases({
    String? start,
    String? end,
    required StatsGranularity granularity,
  }) async {
    final json = await _api.getJson(
      _statsPath('/stats/purchases', start: start, end: end, granularity: granularity),
    );
    return _objects(json, 'purchases').map(PurchasePeriod.fromJson).toList();
  }

  /// 抽出条件と評価の関係を引く (FR-18)。
  Future<List<BrewRating>> brewRatings({
    String? start,
    String? end,
    required int utcOffsetMinutes,
  }) async {
    final json = await _api.getJson(
      _statsPath(
        '/stats/brew-ratings',
        start: start,
        end: end,
        utcOffsetMinutes: utcOffsetMinutes,
      ),
    );
    return _objects(json, 'brew_ratings').map(BrewRating.fromJson).toList();
  }

  /// 購入ごとの評価の推移を引く (FR-18)。期間で絞らず、アーカイブ済みの購入も指定できる。
  Future<List<RatingHistoryEntry>> ratingHistory(String purchaseId) async {
    final json = await _api.getJson('/purchases/$purchaseId/rating-history');
    return _objects(json, 'ratings').map(RatingHistoryEntry.fromJson).toList();
  }

  /// 統計の経路に、期間と粒度と UTC オフセットの指定を付ける。
  ///
  /// 省略したパラメータは付けない (全期間は開始日と終了日の両方を省略する)。
  /// 抽出条件と評価の関係は粒度を持たない。
  static String _statsPath(
    String path, {
    String? start,
    String? end,
    StatsGranularity? granularity,
    int? utcOffsetMinutes,
  }) {
    final params = <String, String>{};
    if (granularity != null) {
      params['granularity'] = granularity.apiValue;
    }
    if (start != null) {
      params['start'] = start;
    }
    if (end != null) {
      params['end'] = end;
    }
    if (utcOffsetMinutes != null) {
      params['utc_offset_minutes'] = '$utcOffsetMinutes';
    }
    return Uri(path: path, queryParameters: params).toString();
  }

  /// 応答の配列の項目を読む。配列でなければ応答の形式の違反にする。
  static List<Map<String, Object?>> _objects(Map<String, Object?> json, String key) {
    final value = json[key];
    if (value is List<Object?>) {
      return value.whereType<Map<String, Object?>>().toList();
    }
    throw FormatException('the $key field must be an array but was $value');
  }
}
