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
  String get homeDescription => 'You are logged in.';

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
}
