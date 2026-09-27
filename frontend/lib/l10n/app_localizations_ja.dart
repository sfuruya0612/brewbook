// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Japanese (`ja`).
class AppLocalizationsJa extends AppLocalizations {
  AppLocalizationsJa([String locale = 'ja']) : super(locale);

  @override
  String get appTitle => 'brewbook';

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
  String get homeTitle => 'brewbook';

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

  @override
  String get menuTooltip => 'メニュー';

  @override
  String get newBrewButton => '抽出を記録';

  @override
  String get brewNewTitle => '抽出を記録';

  @override
  String get brewEditTitle => '抽出を編集';

  @override
  String get brewDetailTitle => '抽出の詳細';

  @override
  String get purchasesTitle => '購入';

  @override
  String get newPurchaseButton => '購入を記録';

  @override
  String get purchaseNewTitle => '購入を記録';

  @override
  String get purchaseEditTitle => '購入を編集';

  @override
  String get purchaseDetailTitle => '購入の詳細';

  @override
  String get productsTitle => '商品';

  @override
  String get newProductButton => '商品を登録';

  @override
  String get productNewTitle => '商品を登録';

  @override
  String get productEditTitle => '商品を編集';

  @override
  String get shopsTitle => '店';

  @override
  String get newShopButton => '店を登録';

  @override
  String get shopNewTitle => '店を登録';

  @override
  String get shopEditTitle => '店を編集';

  @override
  String get saveButton => '保存';

  @override
  String get cancelButton => 'キャンセル';

  @override
  String get editButton => '編集';

  @override
  String get deleteButton => '削除';

  @override
  String get archiveButton => 'アーカイブ';

  @override
  String get unarchiveButton => 'アーカイブ解除';

  @override
  String get selectButton => '選択';

  @override
  String get addButton => '追加';

  @override
  String get includeArchivedLabel => 'アーカイブ済みを含める';

  @override
  String get rowSubtitleSeparator => ' / ';

  @override
  String get noRecords => '記録がありません';

  @override
  String get unsetLabel => '未設定';

  @override
  String get savedMessage => '保存しました。';

  @override
  String get archivedMessage => 'アーカイブしました。';

  @override
  String get unarchivedMessage => 'アーカイブ解除しました。';

  @override
  String get shopNameLabel => '店名';

  @override
  String get productNameLabel => '商品名';

  @override
  String get addressLabel => '住所';

  @override
  String get purchasedOnLabel => '購入日';

  @override
  String get brewedAtLabel => '抽出日時';

  @override
  String get priceLabel => '価格';

  @override
  String get currencyLabel => '通貨';

  @override
  String get weightLabel => '重量';

  @override
  String get doseLabel => '豆の量';

  @override
  String get waterLabel => '湯量';

  @override
  String get waterTempLabel => '湯の温度';

  @override
  String get brewTimeLabel => '時間';

  @override
  String get methodLabel => '抽出方法';

  @override
  String get grindSettingLabel => '挽き目';

  @override
  String get ratingLabel => '評価';

  @override
  String get notesLabel => '感想';

  @override
  String get productLabel => '商品';

  @override
  String get shopLabel => '店';

  @override
  String get purchaseLabel => '購入';

  @override
  String get tagInputHint => 'タグを入力';

  @override
  String get tagInputLabel => 'タグの名前';

  @override
  String get dayFormatHint => 'YYYY-MM-DD';

  @override
  String get timeFormatHint => 'HH:MM';

  @override
  String get photoLabel => '写真';

  @override
  String get photoSelectButton => '写真を選ぶ';

  @override
  String get photoReplaceButton => '写真を差し替え';

  @override
  String get photoDeleteButton => '写真を削除';

  @override
  String get photoNoneLabel => '写真はありません';

  @override
  String get ratingHistoryTitle => '評価の推移';

  @override
  String get selectPurchaseTitle => '購入を選ぶ';

  @override
  String get selectProductTitle => '商品を選ぶ';

  @override
  String get selectShopTitle => '店を選ぶ';

  @override
  String get shopNoneLabel => '店を指定しない';

  @override
  String get validationRequired => '必須の項目です。';

  @override
  String get validationNumber => '0 以上の整数を入力してください。';

  @override
  String get validationDecimal => '0 以上で小数第 1 位までの数を入力してください。';

  @override
  String get validationDay => '日付を YYYY-MM-DD で入力してください。';

  @override
  String get validationTime => '時刻を HH:MM で入力してください。';

  @override
  String get validationProduct => '商品を選んでください。';

  @override
  String get validationPurchase => '購入を選んでください。';

  @override
  String brewRowSubtitle(String date, String shop) {
    return '$date / $shop';
  }

  @override
  String brewRowSubtitleNoShop(String date) {
    return '$date';
  }

  @override
  String gramsValue(String value) {
    return '$value g';
  }

  @override
  String get gramUnit => 'g';

  @override
  String get celsiusUnit => '℃';

  @override
  String get secondUnit => '秒';

  @override
  String celsiusValue(String value) {
    return '$value ℃';
  }

  @override
  String secondsValue(String value) {
    return '$value 秒';
  }

  @override
  String ratingValue(String value) {
    return '$value / 5';
  }

  @override
  String priceValue(String amount, String currency) {
    return '$amount $currency';
  }

  @override
  String get validationCurrency => '通貨コードを ISO 4217 の 3 文字の英大文字で入力してください。';

  @override
  String get statsTitle => '統計';

  @override
  String get statsPeriodCurrentMonth => '当月';

  @override
  String get statsPeriodThreeMonths => '3 か月';

  @override
  String get statsPeriodSixMonths => '6 か月';

  @override
  String get statsPeriodTwelveMonths => '12 か月';

  @override
  String get statsPeriodAllTime => '全期間';

  @override
  String get statsPeriodCustom => '任意';

  @override
  String get statsStartLabel => '開始日';

  @override
  String get statsEndLabel => '終了日';

  @override
  String get statsApplyButton => '適用';

  @override
  String get validationPeriod => '開始日以降の日付を入力してください。';

  @override
  String get statsBrewCountTitle => '抽出回数';

  @override
  String get statsBrewDoseTitle => '豆の消費量';

  @override
  String statsPurchaseAmountTitle(String currency) {
    return '購入金額 ($currency)';
  }

  @override
  String statsPurchaseWeightTitle(String currency) {
    return '購入重量 ($currency)';
  }

  @override
  String get statsCurrencyNone => '通貨なし';

  @override
  String get statsRatingDoseTitle => '豆の量と評価';

  @override
  String get statsRatingWaterTitle => '湯量と評価';

  @override
  String get statsRatingTempTitle => '湯の温度と評価';

  @override
  String get statsRatingTimeTitle => '時間と評価';

  @override
  String get settingsTitle => '設定';

  @override
  String get passkeysTitle => 'パスキー';

  @override
  String get passkeysDescription => 'ログインに使うパスキーを追加したり削除したりできます。';

  @override
  String get addPasskeyTitle => 'パスキーを追加';

  @override
  String get addPasskeyButton => 'パスキーを追加';

  @override
  String get renamePasskeyTitle => 'パスキーの名前を変更';

  @override
  String get renameButton => '名前を変更';

  @override
  String passkeyCreatedAt(String timestamp) {
    return '登録: $timestamp';
  }

  @override
  String passkeyLastUsedAt(String timestamp) {
    return '最終使用: $timestamp';
  }

  @override
  String get passkeyNotUsedYet => 'まだ使われていません';

  @override
  String get passkeyAddedMessage => 'パスキーを追加しました。';

  @override
  String get passkeyRenamedMessage => 'パスキーの名前を変更しました。';

  @override
  String get passkeyDeletedMessage => 'パスキーを削除しました。';

  @override
  String get passkeyLastDeleteError =>
      '最後の 1 つのパスキーは削除できません。先に別のパスキーを追加してください。';

  @override
  String get exportTitle => 'エクスポート';

  @override
  String get exportDescription => '全記録を 1 つの JSON ファイルとしてダウンロードできます。';

  @override
  String get exportButton => 'エクスポートをダウンロード';

  @override
  String get exportDoneMessage => 'エクスポートをダウンロードしました。';

  @override
  String get deleteAccountTitle => 'アカウントの削除';

  @override
  String get deleteAccountDescription => 'アカウントと全記録を削除します。元に戻せません。';

  @override
  String get deleteAccountButton => 'アカウントを削除';

  @override
  String get deleteConfirmButton => '削除する';

  @override
  String get deleteAccountConfirmTitle => 'アカウントを削除しますか？';

  @override
  String get deleteAccountConfirmMessage => 'アカウントと全記録を削除します。元に戻せません。';

  @override
  String get requiredLabel => '必須';

  @override
  String get brewsLabel => '抽出';

  @override
  String get usedBeansLabel => '使った豆';

  @override
  String get archivedBadge => 'アーカイブ済み';

  @override
  String get addressUnset => '住所は未設定';

  @override
  String get ratingNone => '未評価';

  @override
  String get homeEmptyHint => '右下の「抽出を記録」から最初の 1 杯を記録します。';

  @override
  String get purchasesEmptyHint => '右下の「購入を記録」から最初の 1 件を記録します。';

  @override
  String get productsEmptyHint => '右下の「商品を登録」から最初の 1 件を登録します。';

  @override
  String get shopsEmptyHint => '右下の「店を登録」から最初の 1 件を登録します。';

  @override
  String get purchasePickPlaceholder => '購入を選ぶ';

  @override
  String get grindSettingHint => '例: Comandante 24';

  @override
  String get photoConvertNote => 'JPEG に変換し、長辺 2048 px 以下に縮小して保存します。';

  @override
  String get uploadingLabel => 'アップロード中';

  @override
  String get loginRegisterHint => 'ログインできないときは管理者から登録用のリンクを受け取ってください。';

  @override
  String get registerTokenGuidance => '管理者に連絡して、登録用のリンクを再発行してもらってください。';

  @override
  String get statsRangePrefix => '';

  @override
  String get statsRangeMiddle => ' から ';

  @override
  String statsRangeSuffix(String granularity) {
    return ' まで、$granularity';
  }

  @override
  String statsRangeAllTime(String granularity) {
    return '全期間、$granularity';
  }

  @override
  String get statsGranularityDaily => '日別';

  @override
  String get statsGranularityMonthly => '月別';

  @override
  String statsCustomGranularityNote(int days, String granularity) {
    return '$days 日なので$granularityで表示します (62 日以下なら日別)。';
  }

  @override
  String statsChartUnit(String unit, String period) {
    return '$unit / $period';
  }

  @override
  String get statsUnitCups => '杯';

  @override
  String get statsPeriodDay => '日';

  @override
  String get statsPeriodMonth => '月';

  @override
  String get statsScatterSection => '抽出条件と評価';

  @override
  String get statsPurchaseAmountLabel => '購入金額';

  @override
  String get statsPurchaseWeightLabel => '購入重量';
}
