/// パスキーの操作の失敗の種類。
enum PasskeyErrorKind {
  /// 利用者が操作を取り消した (W3C WebAuthn Level 3 の `NotAllowedError` など)。
  cancelled,

  /// この環境ではパスキーを使えない (Web 以外の実装がまだ無いなど)。
  unsupported,

  /// その他の失敗。
  failed,
}

/// パスキーの操作の失敗。
class PasskeyError implements Exception {
  const PasskeyError(this.kind, this.cause);

  /// 失敗の種類。
  final PasskeyErrorKind kind;

  /// 失敗の原因 (JS の例外など)。
  final Object? cause;

  @override
  String toString() => 'PasskeyError(${kind.name}, cause: $cause)';
}
