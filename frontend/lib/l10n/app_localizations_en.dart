// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appTitle => 'coffee-log';

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
  String get homeTitle => 'coffee-log';

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
  String purchaseRowSubtitle(String date, String product) {
    return '$date / $product';
  }

  @override
  String gramsValue(String value) {
    return '$value g';
  }

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
  String get validationCurrency =>
      'Enter the ISO 4217 currency code (three uppercase letters).';
}
