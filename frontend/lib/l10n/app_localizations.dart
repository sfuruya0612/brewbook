import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_ja.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
        delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('en'),
    Locale('ja'),
  ];

  /// No description provided for @appTitle.
  ///
  /// In ja, this message translates to:
  /// **'coffee-log'**
  String get appTitle;

  /// No description provided for @loading.
  ///
  /// In ja, this message translates to:
  /// **'読み込み中'**
  String get loading;

  /// No description provided for @loginTitle.
  ///
  /// In ja, this message translates to:
  /// **'ログイン'**
  String get loginTitle;

  /// No description provided for @loginDescription.
  ///
  /// In ja, this message translates to:
  /// **'パスキーでログインします。利用者名とパスワードの入力は要りません。'**
  String get loginDescription;

  /// No description provided for @loginButton.
  ///
  /// In ja, this message translates to:
  /// **'パスキーでログイン'**
  String get loginButton;

  /// No description provided for @loginFailed.
  ///
  /// In ja, this message translates to:
  /// **'ログインできませんでした。もう一度お試しください。'**
  String get loginFailed;

  /// No description provided for @logoutButton.
  ///
  /// In ja, this message translates to:
  /// **'ログアウト'**
  String get logoutButton;

  /// No description provided for @registerTitle.
  ///
  /// In ja, this message translates to:
  /// **'パスキーの登録'**
  String get registerTitle;

  /// No description provided for @registerDescription.
  ///
  /// In ja, this message translates to:
  /// **'パスキーに付ける名前を入力して登録します。'**
  String get registerDescription;

  /// No description provided for @passkeyNameLabel.
  ///
  /// In ja, this message translates to:
  /// **'パスキーの名前'**
  String get passkeyNameLabel;

  /// No description provided for @passkeyNameHint.
  ///
  /// In ja, this message translates to:
  /// **'例: 自宅の PC'**
  String get passkeyNameHint;

  /// No description provided for @passkeyNameHelper.
  ///
  /// In ja, this message translates to:
  /// **'1 文字以上 50 文字以下で入力してください。'**
  String get passkeyNameHelper;

  /// No description provided for @passkeyNameError.
  ///
  /// In ja, this message translates to:
  /// **'名前を 1 文字以上 50 文字以下で入力してください。'**
  String get passkeyNameError;

  /// No description provided for @registerButton.
  ///
  /// In ja, this message translates to:
  /// **'登録する'**
  String get registerButton;

  /// No description provided for @registerTokenMissing.
  ///
  /// In ja, this message translates to:
  /// **'登録用のトークンが URL にありません。'**
  String get registerTokenMissing;

  /// No description provided for @registerTokenNotFound.
  ///
  /// In ja, this message translates to:
  /// **'登録用のリンクが正しくありません。'**
  String get registerTokenNotFound;

  /// No description provided for @registerTokenUsed.
  ///
  /// In ja, this message translates to:
  /// **'この登録用のリンクは使用済みです。'**
  String get registerTokenUsed;

  /// No description provided for @registerTokenExpired.
  ///
  /// In ja, this message translates to:
  /// **'この登録用のリンクの有効期限が切れています。'**
  String get registerTokenExpired;

  /// No description provided for @homeTitle.
  ///
  /// In ja, this message translates to:
  /// **'coffee-log'**
  String get homeTitle;

  /// No description provided for @homeDescription.
  ///
  /// In ja, this message translates to:
  /// **'ログインしています。'**
  String get homeDescription;

  /// No description provided for @retryButton.
  ///
  /// In ja, this message translates to:
  /// **'再試行'**
  String get retryButton;

  /// No description provided for @errorValidation.
  ///
  /// In ja, this message translates to:
  /// **'入力の内容を確認してください。'**
  String get errorValidation;

  /// No description provided for @errorUnauthorized.
  ///
  /// In ja, this message translates to:
  /// **'セッションが無効になりました。もう一度ログインしてください。'**
  String get errorUnauthorized;

  /// No description provided for @errorNotFound.
  ///
  /// In ja, this message translates to:
  /// **'対象が見つかりません。'**
  String get errorNotFound;

  /// No description provided for @errorConflict.
  ///
  /// In ja, this message translates to:
  /// **'処理できない状態です。画面を開き直してください。'**
  String get errorConflict;

  /// No description provided for @errorGone.
  ///
  /// In ja, this message translates to:
  /// **'有効期限が切れています。'**
  String get errorGone;

  /// No description provided for @errorNetwork.
  ///
  /// In ja, this message translates to:
  /// **'通信に失敗しました。接続を確認してもう一度お試しください。'**
  String get errorNetwork;

  /// No description provided for @errorUnexpected.
  ///
  /// In ja, this message translates to:
  /// **'予期しないエラーが発生しました。'**
  String get errorUnexpected;

  /// No description provided for @passkeyCancelled.
  ///
  /// In ja, this message translates to:
  /// **'パスキーの操作が取り消されました。'**
  String get passkeyCancelled;

  /// No description provided for @passkeyUnsupported.
  ///
  /// In ja, this message translates to:
  /// **'この環境ではパスキーを使えません。'**
  String get passkeyUnsupported;

  /// 商品の項目名 (PRD の FR-16)
  ///
  /// In ja, this message translates to:
  /// **'生産者'**
  String get producer;

  /// 商品の項目名 (PRD の FR-16)
  ///
  /// In ja, this message translates to:
  /// **'生産国'**
  String get origin;

  /// 商品の項目名 (PRD の FR-16)
  ///
  /// In ja, this message translates to:
  /// **'地域'**
  String get region;

  /// 商品の項目名 (PRD の FR-16)
  ///
  /// In ja, this message translates to:
  /// **'精製方法'**
  String get process;

  /// 商品の項目名 (PRD の FR-16)
  ///
  /// In ja, this message translates to:
  /// **'品種'**
  String get variety;

  /// 購入の項目名 (PRD の FR-16)
  ///
  /// In ja, this message translates to:
  /// **'焙煎度'**
  String get roast;

  /// 購入の項目名 (PRD の FR-16)
  ///
  /// In ja, this message translates to:
  /// **'焙煎日'**
  String get roastDate;

  /// 商品の項目名 (PRD の FR-16)
  ///
  /// In ja, this message translates to:
  /// **'フレーバーノート'**
  String get flavorNotes;
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['en', 'ja'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return AppLocalizationsEn();
    case 'ja':
      return AppLocalizationsJa();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
