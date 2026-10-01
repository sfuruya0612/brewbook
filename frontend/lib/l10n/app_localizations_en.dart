// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appTitle => 'brewbook';

  @override
  String get loading => 'Loading';

  @override
  String get loginTitle => 'Log in';

  @override
  String get loginDescription =>
      'Log in with a passkey. No user name or password is needed.';

  @override
  String get loginButton => 'Log in with a passkey';

  @override
  String get loginFailed => 'Could not log in. Please try again.';

  @override
  String get logoutButton => 'Log out';

  @override
  String get registerTitle => 'Register a passkey';

  @override
  String get registerDescription =>
      'Enter a name for the passkey and register it.';

  @override
  String get passkeyNameLabel => 'Passkey name';

  @override
  String get passkeyNameHint => 'e.g. Home PC';

  @override
  String get passkeyNameHelper => 'Use 1 to 50 characters.';

  @override
  String get passkeyNameError => 'Enter a name of 1 to 50 characters.';

  @override
  String get registerButton => 'Register';

  @override
  String get registerTokenMissing =>
      'The URL does not have a registration token.';

  @override
  String get registerTokenNotFound => 'The registration link is not valid.';

  @override
  String get registerTokenUsed =>
      'This registration link has already been used.';

  @override
  String get registerTokenExpired => 'This registration link has expired.';

  @override
  String get homeTitle => 'brewbook';

  @override
  String get retryButton => 'Retry';

  @override
  String get errorValidation => 'Check the input.';

  @override
  String get errorUnauthorized =>
      'The session is no longer valid. Please log in again.';

  @override
  String get errorNotFound => 'The item was not found.';

  @override
  String get errorConflict =>
      'The request cannot be processed. Reload the screen.';

  @override
  String get errorGone => 'The item has expired.';

  @override
  String get errorNetwork =>
      'The connection failed. Check your network and try again.';

  @override
  String get errorUnexpected => 'An unexpected error occurred.';

  @override
  String get passkeyCancelled => 'The passkey operation was cancelled.';

  @override
  String get passkeyUnsupported =>
      'Passkeys are not available in this environment.';

  @override
  String get producer => 'Producer';

  @override
  String get origin => 'Origin';

  @override
  String get region => 'Region';

  @override
  String get process => 'Process';

  @override
  String get variety => 'Variety';

  @override
  String get roast => 'Roast';

  @override
  String get roastDate => 'Roast Date';

  @override
  String get flavorNotes => 'Flavor Notes';

  @override
  String get menuTooltip => 'Menu';

  @override
  String get newBrewButton => 'Log a brew';

  @override
  String get brewNewTitle => 'Log a brew';

  @override
  String get brewEditTitle => 'Edit the brew';

  @override
  String get brewDetailTitle => 'Brew';

  @override
  String get purchasesTitle => 'Purchases';

  @override
  String get newPurchaseButton => 'Log a purchase';

  @override
  String get purchaseNewTitle => 'Log a purchase';

  @override
  String get purchaseEditTitle => 'Edit the purchase';

  @override
  String get purchaseDetailTitle => 'Purchase';

  @override
  String get productsTitle => 'Products';

  @override
  String get newProductButton => 'Add a product';

  @override
  String get productNewTitle => 'Add a product';

  @override
  String get productEditTitle => 'Edit the product';

  @override
  String get shopsTitle => 'Shops';

  @override
  String get newShopButton => 'Add a shop';

  @override
  String get shopNewTitle => 'Add a shop';

  @override
  String get shopEditTitle => 'Edit the shop';

  @override
  String get saveButton => 'Save';

  @override
  String get cancelButton => 'Cancel';

  @override
  String get editButton => 'Edit';

  @override
  String get deleteButton => 'Delete';

  @override
  String get archiveButton => 'Archive';

  @override
  String get unarchiveButton => 'Unarchive';

  @override
  String get selectButton => 'Select';

  @override
  String get addButton => 'Add';

  @override
  String get includeArchivedLabel => 'Include archived';

  @override
  String get rowSubtitleSeparator => ' / ';

  @override
  String get noRecords => 'No records';

  @override
  String get unsetLabel => 'Not set';

  @override
  String get savedMessage => 'Saved.';

  @override
  String get archivedMessage => 'Archived.';

  @override
  String get unarchivedMessage => 'Unarchived.';

  @override
  String get shopNameLabel => 'Shop name';

  @override
  String get productNameLabel => 'Product name';

  @override
  String get addressLabel => 'Address';

  @override
  String get purchasedOnLabel => 'Purchased on';

  @override
  String get brewedAtLabel => 'Brewed at';

  @override
  String get priceLabel => 'Price';

  @override
  String get currencyLabel => 'Currency';

  @override
  String get currencyAed => 'UAE Dirham';

  @override
  String get currencyAud => 'Australian Dollar';

  @override
  String get currencyBrl => 'Brazilian Real';

  @override
  String get currencyCad => 'Canadian Dollar';

  @override
  String get currencyChf => 'Swiss Franc';

  @override
  String get currencyCny => 'Chinese Yuan';

  @override
  String get currencyCzk => 'Czech Koruna';

  @override
  String get currencyDkk => 'Danish Krone';

  @override
  String get currencyEur => 'Euro';

  @override
  String get currencyGbp => 'British Pound';

  @override
  String get currencyHkd => 'Hong Kong Dollar';

  @override
  String get currencyIdr => 'Indonesian Rupiah';

  @override
  String get currencyInr => 'Indian Rupee';

  @override
  String get currencyJpy => 'Japanese Yen';

  @override
  String get currencyKrw => 'South Korean Won';

  @override
  String get currencyMxn => 'Mexican Peso';

  @override
  String get currencyMyr => 'Malaysian Ringgit';

  @override
  String get currencyNok => 'Norwegian Krone';

  @override
  String get currencyNzd => 'New Zealand Dollar';

  @override
  String get currencyPhp => 'Philippine Peso';

  @override
  String get currencyPln => 'Polish Zloty';

  @override
  String get currencySar => 'Saudi Riyal';

  @override
  String get currencySek => 'Swedish Krona';

  @override
  String get currencySgd => 'Singapore Dollar';

  @override
  String get currencyThb => 'Thai Baht';

  @override
  String get currencyTry => 'Turkish Lira';

  @override
  String get currencyTwd => 'New Taiwan Dollar';

  @override
  String get currencyUsd => 'US Dollar';

  @override
  String get currencyVnd => 'Vietnamese Dong';

  @override
  String get currencyZar => 'South African Rand';

  @override
  String get weightLabel => 'Weight';

  @override
  String get doseLabel => 'Dose';

  @override
  String get waterLabel => 'Water';

  @override
  String get waterTempLabel => 'Water temperature';

  @override
  String get brewTimeLabel => 'Time';

  @override
  String get methodLabel => 'Method';

  @override
  String get grindSettingLabel => 'Grind setting';

  @override
  String get ratingLabel => 'Rating';

  @override
  String get notesLabel => 'Notes';

  @override
  String get productLabel => 'Product';

  @override
  String get shopLabel => 'Shop';

  @override
  String get purchaseLabel => 'Purchase';

  @override
  String get tagInputHint => 'Enter a tag';

  @override
  String get tagInputLabel => 'Tag name';

  @override
  String get dayFormatHint => 'YYYY-MM-DD';

  @override
  String get timeFormatHint => 'HH:MM';

  @override
  String get photoLabel => 'Photo';

  @override
  String get photoSelectButton => 'Choose a photo';

  @override
  String get photoReplaceButton => 'Replace the photo';

  @override
  String get photoDeleteButton => 'Delete the photo';

  @override
  String get photoNoneLabel => 'No photo';

  @override
  String get ratingHistoryTitle => 'Rating history';

  @override
  String get selectPurchaseTitle => 'Choose a purchase';

  @override
  String get selectProductTitle => 'Choose a product';

  @override
  String get selectShopTitle => 'Choose a shop';

  @override
  String get shopNoneLabel => 'No shop';

  @override
  String get validationRequired => 'This field is required.';

  @override
  String get validationNumber => 'Enter an integer of 0 or more.';

  @override
  String get validationDecimal =>
      'Enter a number of 0 or more with at most one decimal place.';

  @override
  String get validationDay => 'Enter the date as YYYY-MM-DD.';

  @override
  String get validationTime => 'Enter the time as HH:MM.';

  @override
  String get validationProduct => 'Choose a product.';

  @override
  String get validationPurchase => 'Choose a purchase.';

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
  String get celsiusUnit => '°C';

  @override
  String get secondUnit => 's';

  @override
  String celsiusValue(String value) {
    return '$value °C';
  }

  @override
  String secondsValue(String value) {
    return '$value s';
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
  String get statsTitle => 'Stats';

  @override
  String get statsPeriodCurrentMonth => 'This month';

  @override
  String get statsPeriodThreeMonths => '3 months';

  @override
  String get statsPeriodSixMonths => '6 months';

  @override
  String get statsPeriodTwelveMonths => '12 months';

  @override
  String get statsPeriodAllTime => 'All time';

  @override
  String get statsPeriodCustom => 'Custom';

  @override
  String get statsStartLabel => 'Start date';

  @override
  String get statsEndLabel => 'End date';

  @override
  String get statsApplyButton => 'Apply';

  @override
  String get validationPeriod => 'Enter a date on or after the start date.';

  @override
  String get statsBrewCountTitle => 'Brew count';

  @override
  String get statsBrewDoseTitle => 'Dose';

  @override
  String statsPurchaseAmountTitle(String currency) {
    return 'Purchase amount ($currency)';
  }

  @override
  String statsPurchaseWeightTitle(String currency) {
    return 'Purchase weight ($currency)';
  }

  @override
  String get statsCurrencyNone => 'No currency';

  @override
  String get statsRatingDoseTitle => 'Dose and rating';

  @override
  String get statsRatingWaterTitle => 'Water and rating';

  @override
  String get statsRatingTempTitle => 'Water temperature and rating';

  @override
  String get statsRatingTimeTitle => 'Time and rating';

  @override
  String get settingsTitle => 'Settings';

  @override
  String get passkeysTitle => 'Passkeys';

  @override
  String get passkeysDescription =>
      'Add or remove the passkeys you use to log in.';

  @override
  String get addPasskeyTitle => 'Add a passkey';

  @override
  String get addPasskeyButton => 'Add a passkey';

  @override
  String get renamePasskeyTitle => 'Rename the passkey';

  @override
  String get renameButton => 'Rename';

  @override
  String passkeyCreatedAt(String timestamp) {
    return 'Created: $timestamp';
  }

  @override
  String passkeyLastUsedAt(String timestamp) {
    return 'Last used: $timestamp';
  }

  @override
  String get passkeyNotUsedYet => 'Not used yet';

  @override
  String get passkeyAddedMessage => 'Added the passkey.';

  @override
  String get passkeyRenamedMessage => 'Renamed the passkey.';

  @override
  String get passkeyDeletedMessage => 'Deleted the passkey.';

  @override
  String get passkeyLastDeleteError =>
      'The last passkey cannot be deleted. Add another passkey first.';

  @override
  String get exportTitle => 'Export';

  @override
  String get exportDescription =>
      'Download all of your records as a single JSON file.';

  @override
  String get exportButton => 'Download the export';

  @override
  String get exportDoneMessage => 'Downloaded the export.';

  @override
  String get deleteAccountTitle => 'Delete the account';

  @override
  String get deleteAccountDescription =>
      'Deletes your account and all of your records. This cannot be undone.';

  @override
  String get deleteAccountButton => 'Delete the account';

  @override
  String get deleteConfirmButton => 'Delete';

  @override
  String get deleteAccountConfirmTitle => 'Delete the account?';

  @override
  String get deleteAccountConfirmMessage =>
      'Your account and all of your records will be deleted. This cannot be undone.';

  @override
  String get requiredLabel => 'Required';

  @override
  String get brewsLabel => 'Brews';

  @override
  String get usedBeansLabel => 'Beans used';

  @override
  String get archivedBadge => 'Archived';

  @override
  String get addressUnset => 'No address';

  @override
  String get ratingNone => 'Not rated';

  @override
  String get homeEmptyHint =>
      'Use the button at the bottom right to log your first cup.';

  @override
  String get purchasesEmptyHint =>
      'Use the button at the bottom right to log your first purchase.';

  @override
  String get productsEmptyHint =>
      'Use the button at the bottom right to add your first product.';

  @override
  String get shopsEmptyHint =>
      'Use the button at the bottom right to add your first shop.';

  @override
  String get purchasePickPlaceholder => 'Choose a purchase';

  @override
  String get grindSettingHint => 'e.g. Comandante 24';

  @override
  String get photoConvertNote =>
      'Converts to JPEG and saves at a longest side of 2048 px or less.';

  @override
  String get uploadingLabel => 'Uploading';

  @override
  String get suggestionLoadingLabel => 'Reading the photo';

  @override
  String get suggestionFailedMessage =>
      'Could not read the photo. You can continue by entering the values manually.';

  @override
  String get suggestionRegisterProductButton =>
      'Add a product with the suggested values';

  @override
  String get loginRegisterHint =>
      'If you cannot log in, ask the administrator for a registration link.';

  @override
  String get registerTokenGuidance =>
      'Contact the administrator to have a new registration link issued.';

  @override
  String get statsRangePrefix => 'From ';

  @override
  String get statsRangeMiddle => ' to ';

  @override
  String statsRangeSuffix(String granularity) {
    return ', $granularity';
  }

  @override
  String statsRangeAllTime(String granularity) {
    return 'All time, $granularity';
  }

  @override
  String get statsGranularityDaily => 'daily';

  @override
  String get statsGranularityMonthly => 'monthly';

  @override
  String statsCustomGranularityNote(int days, String granularity) {
    return '$days days, so showing $granularity (daily if 62 days or fewer).';
  }

  @override
  String statsChartUnit(String unit, String period) {
    return '$unit / $period';
  }

  @override
  String get statsUnitCups => 'cups';

  @override
  String get statsPeriodDay => 'day';

  @override
  String get statsPeriodMonth => 'month';

  @override
  String get statsScatterSection => 'Brew conditions and rating';

  @override
  String get statsPurchaseAmountLabel => 'Purchase amount';

  @override
  String get statsPurchaseWeightLabel => 'Purchase weight';
}
