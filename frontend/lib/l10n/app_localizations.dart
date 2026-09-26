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
  /// **'brewbook'**
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
  /// **'brewbook'**
  String get homeTitle;

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

  /// No description provided for @menuTooltip.
  ///
  /// In ja, this message translates to:
  /// **'メニュー'**
  String get menuTooltip;

  /// No description provided for @newBrewButton.
  ///
  /// In ja, this message translates to:
  /// **'抽出を記録'**
  String get newBrewButton;

  /// No description provided for @brewNewTitle.
  ///
  /// In ja, this message translates to:
  /// **'抽出を記録'**
  String get brewNewTitle;

  /// No description provided for @brewEditTitle.
  ///
  /// In ja, this message translates to:
  /// **'抽出を編集'**
  String get brewEditTitle;

  /// No description provided for @brewDetailTitle.
  ///
  /// In ja, this message translates to:
  /// **'抽出の詳細'**
  String get brewDetailTitle;

  /// No description provided for @purchasesTitle.
  ///
  /// In ja, this message translates to:
  /// **'購入'**
  String get purchasesTitle;

  /// No description provided for @newPurchaseButton.
  ///
  /// In ja, this message translates to:
  /// **'購入を記録'**
  String get newPurchaseButton;

  /// No description provided for @purchaseNewTitle.
  ///
  /// In ja, this message translates to:
  /// **'購入を記録'**
  String get purchaseNewTitle;

  /// No description provided for @purchaseEditTitle.
  ///
  /// In ja, this message translates to:
  /// **'購入を編集'**
  String get purchaseEditTitle;

  /// No description provided for @purchaseDetailTitle.
  ///
  /// In ja, this message translates to:
  /// **'購入の詳細'**
  String get purchaseDetailTitle;

  /// No description provided for @productsTitle.
  ///
  /// In ja, this message translates to:
  /// **'商品'**
  String get productsTitle;

  /// No description provided for @newProductButton.
  ///
  /// In ja, this message translates to:
  /// **'商品を登録'**
  String get newProductButton;

  /// No description provided for @productNewTitle.
  ///
  /// In ja, this message translates to:
  /// **'商品を登録'**
  String get productNewTitle;

  /// No description provided for @productEditTitle.
  ///
  /// In ja, this message translates to:
  /// **'商品を編集'**
  String get productEditTitle;

  /// No description provided for @shopsTitle.
  ///
  /// In ja, this message translates to:
  /// **'店'**
  String get shopsTitle;

  /// No description provided for @newShopButton.
  ///
  /// In ja, this message translates to:
  /// **'店を登録'**
  String get newShopButton;

  /// No description provided for @shopNewTitle.
  ///
  /// In ja, this message translates to:
  /// **'店を登録'**
  String get shopNewTitle;

  /// No description provided for @shopEditTitle.
  ///
  /// In ja, this message translates to:
  /// **'店を編集'**
  String get shopEditTitle;

  /// No description provided for @saveButton.
  ///
  /// In ja, this message translates to:
  /// **'保存'**
  String get saveButton;

  /// No description provided for @cancelButton.
  ///
  /// In ja, this message translates to:
  /// **'キャンセル'**
  String get cancelButton;

  /// No description provided for @editButton.
  ///
  /// In ja, this message translates to:
  /// **'編集'**
  String get editButton;

  /// No description provided for @deleteButton.
  ///
  /// In ja, this message translates to:
  /// **'削除'**
  String get deleteButton;

  /// No description provided for @archiveButton.
  ///
  /// In ja, this message translates to:
  /// **'アーカイブ'**
  String get archiveButton;

  /// No description provided for @unarchiveButton.
  ///
  /// In ja, this message translates to:
  /// **'アーカイブ解除'**
  String get unarchiveButton;

  /// No description provided for @selectButton.
  ///
  /// In ja, this message translates to:
  /// **'選択'**
  String get selectButton;

  /// No description provided for @addButton.
  ///
  /// In ja, this message translates to:
  /// **'追加'**
  String get addButton;

  /// No description provided for @includeArchivedLabel.
  ///
  /// In ja, this message translates to:
  /// **'アーカイブ済みを含める'**
  String get includeArchivedLabel;

  /// No description provided for @noRecords.
  ///
  /// In ja, this message translates to:
  /// **'記録がありません'**
  String get noRecords;

  /// No description provided for @unsetLabel.
  ///
  /// In ja, this message translates to:
  /// **'未設定'**
  String get unsetLabel;

  /// No description provided for @savedMessage.
  ///
  /// In ja, this message translates to:
  /// **'保存しました。'**
  String get savedMessage;

  /// No description provided for @archivedMessage.
  ///
  /// In ja, this message translates to:
  /// **'アーカイブしました。'**
  String get archivedMessage;

  /// No description provided for @unarchivedMessage.
  ///
  /// In ja, this message translates to:
  /// **'アーカイブ解除しました。'**
  String get unarchivedMessage;

  /// No description provided for @shopNameLabel.
  ///
  /// In ja, this message translates to:
  /// **'店名'**
  String get shopNameLabel;

  /// No description provided for @productNameLabel.
  ///
  /// In ja, this message translates to:
  /// **'商品名'**
  String get productNameLabel;

  /// No description provided for @addressLabel.
  ///
  /// In ja, this message translates to:
  /// **'住所'**
  String get addressLabel;

  /// No description provided for @purchasedOnLabel.
  ///
  /// In ja, this message translates to:
  /// **'購入日'**
  String get purchasedOnLabel;

  /// No description provided for @brewedAtLabel.
  ///
  /// In ja, this message translates to:
  /// **'抽出日時'**
  String get brewedAtLabel;

  /// No description provided for @priceLabel.
  ///
  /// In ja, this message translates to:
  /// **'価格'**
  String get priceLabel;

  /// No description provided for @currencyLabel.
  ///
  /// In ja, this message translates to:
  /// **'通貨'**
  String get currencyLabel;

  /// No description provided for @weightLabel.
  ///
  /// In ja, this message translates to:
  /// **'重量'**
  String get weightLabel;

  /// No description provided for @doseLabel.
  ///
  /// In ja, this message translates to:
  /// **'豆の量'**
  String get doseLabel;

  /// No description provided for @waterLabel.
  ///
  /// In ja, this message translates to:
  /// **'湯量'**
  String get waterLabel;

  /// No description provided for @waterTempLabel.
  ///
  /// In ja, this message translates to:
  /// **'湯の温度'**
  String get waterTempLabel;

  /// No description provided for @brewTimeLabel.
  ///
  /// In ja, this message translates to:
  /// **'時間'**
  String get brewTimeLabel;

  /// No description provided for @methodLabel.
  ///
  /// In ja, this message translates to:
  /// **'抽出方法'**
  String get methodLabel;

  /// No description provided for @grindSettingLabel.
  ///
  /// In ja, this message translates to:
  /// **'挽き目'**
  String get grindSettingLabel;

  /// No description provided for @ratingLabel.
  ///
  /// In ja, this message translates to:
  /// **'評価'**
  String get ratingLabel;

  /// No description provided for @notesLabel.
  ///
  /// In ja, this message translates to:
  /// **'感想'**
  String get notesLabel;

  /// No description provided for @productLabel.
  ///
  /// In ja, this message translates to:
  /// **'商品'**
  String get productLabel;

  /// No description provided for @shopLabel.
  ///
  /// In ja, this message translates to:
  /// **'店'**
  String get shopLabel;

  /// No description provided for @purchaseLabel.
  ///
  /// In ja, this message translates to:
  /// **'購入'**
  String get purchaseLabel;

  /// No description provided for @tagInputHint.
  ///
  /// In ja, this message translates to:
  /// **'タグを入力'**
  String get tagInputHint;

  /// No description provided for @tagInputLabel.
  ///
  /// In ja, this message translates to:
  /// **'タグの名前'**
  String get tagInputLabel;

  /// No description provided for @dayFormatHint.
  ///
  /// In ja, this message translates to:
  /// **'YYYY-MM-DD'**
  String get dayFormatHint;

  /// No description provided for @timeFormatHint.
  ///
  /// In ja, this message translates to:
  /// **'HH:MM'**
  String get timeFormatHint;

  /// No description provided for @photoLabel.
  ///
  /// In ja, this message translates to:
  /// **'写真'**
  String get photoLabel;

  /// No description provided for @photoSelectButton.
  ///
  /// In ja, this message translates to:
  /// **'写真を選ぶ'**
  String get photoSelectButton;

  /// No description provided for @photoReplaceButton.
  ///
  /// In ja, this message translates to:
  /// **'写真を差し替え'**
  String get photoReplaceButton;

  /// No description provided for @photoDeleteButton.
  ///
  /// In ja, this message translates to:
  /// **'写真を削除'**
  String get photoDeleteButton;

  /// No description provided for @photoNoneLabel.
  ///
  /// In ja, this message translates to:
  /// **'写真はありません'**
  String get photoNoneLabel;

  /// No description provided for @ratingHistoryTitle.
  ///
  /// In ja, this message translates to:
  /// **'評価の推移'**
  String get ratingHistoryTitle;

  /// No description provided for @selectPurchaseTitle.
  ///
  /// In ja, this message translates to:
  /// **'購入を選ぶ'**
  String get selectPurchaseTitle;

  /// No description provided for @selectProductTitle.
  ///
  /// In ja, this message translates to:
  /// **'商品を選ぶ'**
  String get selectProductTitle;

  /// No description provided for @selectShopTitle.
  ///
  /// In ja, this message translates to:
  /// **'店を選ぶ'**
  String get selectShopTitle;

  /// No description provided for @shopNoneLabel.
  ///
  /// In ja, this message translates to:
  /// **'店を指定しない'**
  String get shopNoneLabel;

  /// No description provided for @validationRequired.
  ///
  /// In ja, this message translates to:
  /// **'必須の項目です。'**
  String get validationRequired;

  /// No description provided for @validationNumber.
  ///
  /// In ja, this message translates to:
  /// **'0 以上の整数を入力してください。'**
  String get validationNumber;

  /// No description provided for @validationDecimal.
  ///
  /// In ja, this message translates to:
  /// **'0 以上で小数第 1 位までの数を入力してください。'**
  String get validationDecimal;

  /// No description provided for @validationDay.
  ///
  /// In ja, this message translates to:
  /// **'日付を YYYY-MM-DD で入力してください。'**
  String get validationDay;

  /// No description provided for @validationTime.
  ///
  /// In ja, this message translates to:
  /// **'時刻を HH:MM で入力してください。'**
  String get validationTime;

  /// No description provided for @validationProduct.
  ///
  /// In ja, this message translates to:
  /// **'商品を選んでください。'**
  String get validationProduct;

  /// No description provided for @validationPurchase.
  ///
  /// In ja, this message translates to:
  /// **'購入を選んでください。'**
  String get validationPurchase;

  /// 抽出の一覧の行の補足 (店が無い場合は brewRowSubtitleNoShop)
  ///
  /// In ja, this message translates to:
  /// **'{date} / {shop}'**
  String brewRowSubtitle(String date, String shop);

  /// 抽出の一覧の行の補足 (店が無い場合)
  ///
  /// In ja, this message translates to:
  /// **'{date}'**
  String brewRowSubtitleNoShop(String date);

  /// 購入の一覧の行の補足
  ///
  /// In ja, this message translates to:
  /// **'{date} / {product}'**
  String purchaseRowSubtitle(String date, String product);

  /// グラムの値の表示
  ///
  /// In ja, this message translates to:
  /// **'{value} g'**
  String gramsValue(String value);

  /// 摂氏の値の表示
  ///
  /// In ja, this message translates to:
  /// **'{value} ℃'**
  String celsiusValue(String value);

  /// 秒の値の表示
  ///
  /// In ja, this message translates to:
  /// **'{value} 秒'**
  String secondsValue(String value);

  /// 評価の値の表示
  ///
  /// In ja, this message translates to:
  /// **'{value} / 5'**
  String ratingValue(String value);

  /// 価格の表示
  ///
  /// In ja, this message translates to:
  /// **'{amount} {currency}'**
  String priceValue(String amount, String currency);

  /// No description provided for @validationCurrency.
  ///
  /// In ja, this message translates to:
  /// **'通貨コードを ISO 4217 の 3 文字の英大文字で入力してください。'**
  String get validationCurrency;

  /// 統計画面の見出し (FR-18)
  ///
  /// In ja, this message translates to:
  /// **'統計'**
  String get statsTitle;

  /// 統計の期間の切り替え (FR-18)
  ///
  /// In ja, this message translates to:
  /// **'当月'**
  String get statsPeriodCurrentMonth;

  /// 統計の期間の切り替え (FR-18)
  ///
  /// In ja, this message translates to:
  /// **'3 か月'**
  String get statsPeriodThreeMonths;

  /// 統計の期間の切り替え (FR-18)
  ///
  /// In ja, this message translates to:
  /// **'6 か月'**
  String get statsPeriodSixMonths;

  /// 統計の期間の切り替え (FR-18)
  ///
  /// In ja, this message translates to:
  /// **'12 か月'**
  String get statsPeriodTwelveMonths;

  /// 統計の期間の切り替え (FR-18)
  ///
  /// In ja, this message translates to:
  /// **'全期間'**
  String get statsPeriodAllTime;

  /// 統計の期間の切り替え (FR-18)
  ///
  /// In ja, this message translates to:
  /// **'任意'**
  String get statsPeriodCustom;

  /// 任意の期間の開始日 (FR-18)
  ///
  /// In ja, this message translates to:
  /// **'開始日'**
  String get statsStartLabel;

  /// 任意の期間の終了日 (FR-18)
  ///
  /// In ja, this message translates to:
  /// **'終了日'**
  String get statsEndLabel;

  /// 任意の期間の適用 (FR-18)
  ///
  /// In ja, this message translates to:
  /// **'適用'**
  String get statsApplyButton;

  /// 任意の期間の検証 (FR-18)
  ///
  /// In ja, this message translates to:
  /// **'開始日以降の日付を入力してください。'**
  String get validationPeriod;

  /// 抽出回数と豆の消費量の棒グラフ (FR-18)
  ///
  /// In ja, this message translates to:
  /// **'抽出回数'**
  String get statsBrewCountTitle;

  /// 抽出回数と豆の消費量の棒グラフ (FR-18)
  ///
  /// In ja, this message translates to:
  /// **'豆の消費量'**
  String get statsBrewDoseTitle;

  /// 購入金額の棒グラフの見出し (通貨コードごと。FR-18)
  ///
  /// In ja, this message translates to:
  /// **'購入金額 ({currency})'**
  String statsPurchaseAmountTitle(String currency);

  /// 購入重量の棒グラフの見出し (通貨コードごと。FR-18)
  ///
  /// In ja, this message translates to:
  /// **'購入重量 ({currency})'**
  String statsPurchaseWeightTitle(String currency);

  /// 価格が無い購入の通貨コードの表示 (FR-18)
  ///
  /// In ja, this message translates to:
  /// **'通貨なし'**
  String get statsCurrencyNone;

  /// 抽出条件と評価の関係の散布図 (FR-18)
  ///
  /// In ja, this message translates to:
  /// **'豆の量と評価'**
  String get statsRatingDoseTitle;

  /// 抽出条件と評価の関係の散布図 (FR-18)
  ///
  /// In ja, this message translates to:
  /// **'湯量と評価'**
  String get statsRatingWaterTitle;

  /// 抽出条件と評価の関係の散布図 (FR-18)
  ///
  /// In ja, this message translates to:
  /// **'湯の温度と評価'**
  String get statsRatingTempTitle;

  /// 抽出条件と評価の関係の散布図 (FR-18)
  ///
  /// In ja, this message translates to:
  /// **'時間と評価'**
  String get statsRatingTimeTitle;

  /// No description provided for @settingsTitle.
  ///
  /// In ja, this message translates to:
  /// **'設定'**
  String get settingsTitle;

  /// No description provided for @passkeysTitle.
  ///
  /// In ja, this message translates to:
  /// **'パスキー'**
  String get passkeysTitle;

  /// No description provided for @passkeysDescription.
  ///
  /// In ja, this message translates to:
  /// **'ログインに使うパスキーを追加したり削除したりできます。'**
  String get passkeysDescription;

  /// No description provided for @addPasskeyTitle.
  ///
  /// In ja, this message translates to:
  /// **'パスキーを追加'**
  String get addPasskeyTitle;

  /// No description provided for @addPasskeyButton.
  ///
  /// In ja, this message translates to:
  /// **'パスキーを追加'**
  String get addPasskeyButton;

  /// No description provided for @renamePasskeyTitle.
  ///
  /// In ja, this message translates to:
  /// **'パスキーの名前を変更'**
  String get renamePasskeyTitle;

  /// No description provided for @renameButton.
  ///
  /// In ja, this message translates to:
  /// **'名前を変更'**
  String get renameButton;

  /// パスキーの登録日時の表示
  ///
  /// In ja, this message translates to:
  /// **'登録: {timestamp}'**
  String passkeyCreatedAt(String timestamp);

  /// パスキーの最終使用日時の表示
  ///
  /// In ja, this message translates to:
  /// **'最終使用: {timestamp}'**
  String passkeyLastUsedAt(String timestamp);

  /// No description provided for @passkeyNotUsedYet.
  ///
  /// In ja, this message translates to:
  /// **'まだ使われていません'**
  String get passkeyNotUsedYet;

  /// No description provided for @passkeyAddedMessage.
  ///
  /// In ja, this message translates to:
  /// **'パスキーを追加しました。'**
  String get passkeyAddedMessage;

  /// No description provided for @passkeyRenamedMessage.
  ///
  /// In ja, this message translates to:
  /// **'パスキーの名前を変更しました。'**
  String get passkeyRenamedMessage;

  /// No description provided for @passkeyDeletedMessage.
  ///
  /// In ja, this message translates to:
  /// **'パスキーを削除しました。'**
  String get passkeyDeletedMessage;

  /// No description provided for @passkeyLastDeleteError.
  ///
  /// In ja, this message translates to:
  /// **'最後の 1 つのパスキーは削除できません。先に別のパスキーを追加してください。'**
  String get passkeyLastDeleteError;

  /// No description provided for @exportTitle.
  ///
  /// In ja, this message translates to:
  /// **'エクスポート'**
  String get exportTitle;

  /// No description provided for @exportDescription.
  ///
  /// In ja, this message translates to:
  /// **'全記録を 1 つの JSON ファイルとしてダウンロードできます。'**
  String get exportDescription;

  /// No description provided for @exportButton.
  ///
  /// In ja, this message translates to:
  /// **'エクスポートをダウンロード'**
  String get exportButton;

  /// No description provided for @exportDoneMessage.
  ///
  /// In ja, this message translates to:
  /// **'エクスポートをダウンロードしました。'**
  String get exportDoneMessage;

  /// No description provided for @deleteAccountTitle.
  ///
  /// In ja, this message translates to:
  /// **'アカウントの削除'**
  String get deleteAccountTitle;

  /// No description provided for @deleteAccountDescription.
  ///
  /// In ja, this message translates to:
  /// **'アカウントと全記録を削除します。元に戻せません。'**
  String get deleteAccountDescription;

  /// No description provided for @deleteAccountButton.
  ///
  /// In ja, this message translates to:
  /// **'アカウントを削除'**
  String get deleteAccountButton;

  /// No description provided for @deleteAccountConfirmTitle.
  ///
  /// In ja, this message translates to:
  /// **'アカウントを削除しますか？'**
  String get deleteAccountConfirmTitle;

  /// No description provided for @deleteAccountConfirmMessage.
  ///
  /// In ja, this message translates to:
  /// **'アカウントと全記録を削除します。元に戻せません。'**
  String get deleteAccountConfirmMessage;
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
