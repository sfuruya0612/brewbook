import '../api/api_error.dart';
import '../auth/passkey_error.dart';
import '../l10n/app_localizations.dart';

/// 例外を画面に出す文言に変換する。文言は全て ARB から取る (FR-16)。
///
/// 401 はセッションの失効、ネットワークエラーは再試行の促しにする (ADR-0007)。
String messageForError(Object error, AppLocalizations l10n) {
  switch (error) {
    case ApiError(isUnauthorized: true):
      return l10n.errorUnauthorized;
    case ApiError(status: 400):
      return l10n.errorValidation;
    case ApiError(status: 404):
      return l10n.errorNotFound;
    case ApiError(status: 409):
      return l10n.errorConflict;
    case ApiError(status: 410):
      return l10n.errorGone;
    case ApiError():
      return l10n.errorUnexpected;
    case NetworkError():
      return l10n.errorNetwork;
    case PasskeyError(kind: PasskeyErrorKind.cancelled):
      return l10n.passkeyCancelled;
    case PasskeyError(kind: PasskeyErrorKind.unsupported):
      return l10n.passkeyUnsupported;
    case PasskeyError():
      return l10n.errorUnexpected;
    default:
      return l10n.errorUnexpected;
  }
}

/// 登録の画面の文言 (FR-1)。
///
/// 登録の経路の 404、409、410 は登録用トークンの問題を表すため、専用の文言にする。
/// 400 は名前の不正とクレデンシャルの検証の失敗のどちらでも返る (コードは同じ) ため、
/// 名前だけを指す文言にせず、入力の確認を促す文言にする (名前は画面側でも検証する)。
String registerErrorMessage(Object error, AppLocalizations l10n) {
  if (error is ApiError) {
    return switch (error.status) {
      400 => l10n.errorValidation,
      404 => l10n.registerTokenNotFound,
      409 => l10n.registerTokenUsed,
      410 => l10n.registerTokenExpired,
      _ => messageForError(error, l10n),
    };
  }
  return messageForError(error, l10n);
}

/// ログインの画面の文言 (FR-2)。
///
/// 401 以外の API のエラー (チャレンジの検証の失敗、期限切れ、再利用) は、やり直しで回復する。
String loginErrorMessage(Object error, AppLocalizations l10n) {
  if (error is ApiError && !error.isUnauthorized) {
    return l10n.loginFailed;
  }
  return messageForError(error, l10n);
}

/// パスキーの削除の文言 (FR-3)。
///
/// 409 は最後の 1 つを消せないことを表すため、追加を促す文言にする (ADR-0004)。
String deletePasskeyErrorMessage(Object error, AppLocalizations l10n) {
  if (error is ApiError && error.status == 409) {
    return l10n.passkeyLastDeleteError;
  }
  return messageForError(error, l10n);
}
