/// API の応答のエラー (PRD の「エラー応答の規約」)。
///
/// Backend のエラー応答は `{"error": {"code": "<snake_case>", "message": "<英語>"}}` の形を取る。
/// `code` と `message` はそのまま保持し、`status` で画面の分岐 (401 はログイン画面へ遷移させる) を行う。
class ApiError implements Exception {
  const ApiError({required this.status, required this.code, required this.message});

  /// HTTP のステータスコード。
  final int status;

  /// Backend が返したエラーの種別 (snake_case)。
  final String code;

  /// Backend が返した英語のメッセージ。画面には出さず、ログと原因の特定に使う。
  final String message;

  /// 認証が失われたか (401)。
  bool get isUnauthorized => status == 401;

  @override
  String toString() => 'ApiError(status: $status, code: $code, message: $message)';
}

/// 応答を取得できなかったこと (接続の失敗、応答の形式の違反) を表す。
///
/// 画面は再試行を促す表示にする (ADR-0007 のオンライン前提)。
class NetworkError implements Exception {
  const NetworkError(this.cause);

  /// 失敗の原因 (接続の例外、タイムアウト、JSON の解析の失敗)。
  final Object cause;

  @override
  String toString() => 'NetworkError($cause)';
}
