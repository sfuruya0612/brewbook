// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Japanese (`ja`).
class AppLocalizationsJa extends AppLocalizations {
  AppLocalizationsJa([String locale = 'ja']) : super(locale);

  @override
  String get appTitle => 'coffee-log';

  @override
  String get loading => '読み込み中';

  @override
  String get loginTitle => 'ログイン';

  @override
  String get loginDescription => 'パスキーでログインします。利用者名とパスワードの入力は要りません。';

  @override
  String get loginButton => 'パスキーでログイン';

  @override
  String get loginFailed => 'ログインできませんでした。もう一度お試しください。';

  @override
  String get logoutButton => 'ログアウト';

  @override
  String get registerTitle => 'パスキーの登録';

  @override
  String get registerDescription => 'パスキーに付ける名前を入力して登録します。';

  @override
  String get passkeyNameLabel => 'パスキーの名前';

  @override
  String get passkeyNameHint => '例: 自宅の PC';

  @override
  String get passkeyNameHelper => '1 文字以上 50 文字以下で入力してください。';

  @override
  String get passkeyNameError => '名前を 1 文字以上 50 文字以下で入力してください。';

  @override
  String get registerButton => '登録する';

  @override
  String get registerTokenMissing => '登録用のトークンが URL にありません。';

  @override
  String get registerTokenNotFound => '登録用のリンクが正しくありません。';

  @override
  String get registerTokenUsed => 'この登録用のリンクは使用済みです。';

  @override
  String get registerTokenExpired => 'この登録用のリンクの有効期限が切れています。';

  @override
  String get homeTitle => 'coffee-log';

  @override
  String get homeDescription => 'ログインしています。';

  @override
  String get retryButton => '再試行';

  @override
  String get errorValidation => '入力の内容を確認してください。';

  @override
  String get errorUnauthorized => 'セッションが無効になりました。もう一度ログインしてください。';

  @override
  String get errorNotFound => '対象が見つかりません。';

  @override
  String get errorConflict => '処理できない状態です。画面を開き直してください。';

  @override
  String get errorGone => '有効期限が切れています。';

  @override
  String get errorNetwork => '通信に失敗しました。接続を確認してもう一度お試しください。';

  @override
  String get errorUnexpected => '予期しないエラーが発生しました。';

  @override
  String get passkeyCancelled => 'パスキーの操作が取り消されました。';

  @override
  String get passkeyUnsupported => 'この環境ではパスキーを使えません。';

  @override
  String get producer => '生産者';

  @override
  String get origin => '生産国';

  @override
  String get region => '地域';

  @override
  String get process => '精製方法';

  @override
  String get variety => '品種';

  @override
  String get roast => '焙煎度';

  @override
  String get roastDate => '焙煎日';

  @override
  String get flavorNotes => 'フレーバーノート';
}
