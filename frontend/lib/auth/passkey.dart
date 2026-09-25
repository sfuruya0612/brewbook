/// 登録済みのパスキー (FR-3)。
///
/// 日時は API の形式 (ISO 8601 の UTC) の文字列のまま保持し、表示の直前に端末の
/// タイムゾーンへ変換する (記録の型と同じ扱い)。
class Passkey {
  const Passkey({
    required this.id,
    required this.name,
    required this.createdAt,
    this.lastUsedAt,
  });

  /// JSON のオブジェクト (`GET /api/passkeys` の 1 件) から組み立てる。
  factory Passkey.fromJson(Map<String, Object?> json) {
    return Passkey(
      id: _string(json, 'id'),
      name: _string(json, 'name'),
      createdAt: _string(json, 'created_at'),
      lastUsedAt: _optionalString(json, 'last_used_at'),
    );
  }

  /// パスキーの ID。
  final String id;

  /// 利用者が付けた名前。
  final String name;

  /// 登録日時 (ISO 8601 の UTC)。
  final String createdAt;

  /// 最終使用日時 (ISO 8601 の UTC)。まだ使われていなければ null。
  final String? lastUsedAt;
}

/// パスキーの一覧の応答から、パスキーの配列を読む (FR-3)。
List<Passkey> passkeysFromJson(Map<String, Object?> json) {
  final items = json['passkeys'];
  if (items is! List<Object?>) {
    throw FormatException('the passkeys field must be an array but was $items');
  }
  return items
      .whereType<Map<String, Object?>>()
      .map(Passkey.fromJson)
      .toList();
}

/// 文字列の項目を読む。
String _string(Map<String, Object?> json, String key) {
  final value = json[key];
  if (value is String) {
    return value;
  }
  throw FormatException('the $key field must be a string but was $value');
}

/// 文字列の項目を読む。無い場合と `null` は null にする。
String? _optionalString(Map<String, Object?> json, String key) {
  final value = json[key];
  if (value == null) {
    return null;
  }
  if (value is String) {
    return value;
  }
  throw FormatException('the $key field must be a string but was $value');
}
