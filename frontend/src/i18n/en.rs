//! 英語の表 (FR-16)。
//!
//! `frontend/lib/l10n/app_en.arb` から写す。`EN[key.index()]` で引く。

use super::keys::KEY_COUNT;

/// キーの添字で引く英語の文言。
pub(super) const EN: [&str; KEY_COUNT] = [
    "brewbook",                                                                     // appTitle
    "Loading",                                                                      // loading
    "Log in",                                                                       // loginTitle
    "Log in with a passkey. No user name or password is needed.", // loginDescription
    "Log in with a passkey",                                      // loginButton
    "Could not log in. Please try again.",                        // loginFailed
    "Log out",                                                    // logoutButton
    "Register a passkey",                                         // registerTitle
    "Enter a name for the passkey and register it.",              // registerDescription
    "Passkey name",                                               // passkeyNameLabel
    "e.g. Home PC",                                               // passkeyNameHint
    "Use 1 to 50 characters.",                                    // passkeyNameHelper
    "Enter a name of 1 to 50 characters.",                        // passkeyNameError
    "Register",                                                   // registerButton
    "The URL does not have a registration token.",                // registerTokenMissing
    "The registration link is not valid.",                        // registerTokenNotFound
    "This registration link has already been used.",              // registerTokenUsed
    "This registration link has expired.",                        // registerTokenExpired
    "brewbook",                                                   // homeTitle
    "Retry",                                                      // retryButton
    "Check the input.",                                           // errorValidation
    "The session is no longer valid. Please log in again.",       // errorUnauthorized
    "The item was not found.",                                    // errorNotFound
    "The request cannot be processed. Reload the screen.",        // errorConflict
    "The item has expired.",                                      // errorGone
    "The connection failed. Check your network and try again.",   // errorNetwork
    "An unexpected error occurred.",                              // errorUnexpected
    "The passkey operation was cancelled.",                       // passkeyCancelled
    "Passkeys are not available in this environment.",            // passkeyUnsupported
    "Producer",                                                   // producer
    "Origin",                                                     // origin
    "Region",                                                     // region
    "Process",                                                    // process
    "Variety",                                                    // variety
    "Roast",                                                      // roast
    "Roast Date",                                                 // roastDate
    "Flavor Notes",                                               // flavorNotes
    "Menu",                                                       // menuTooltip
    "Log a brew",                                                 // newBrewButton
    "Log a brew",                                                 // brewNewTitle
    "Edit the brew",                                              // brewEditTitle
    "Brew",                                                       // brewDetailTitle
    "Purchases",                                                  // purchasesTitle
    "Log a purchase",                                             // newPurchaseButton
    "Log a purchase",                                             // purchaseNewTitle
    "Edit the purchase",                                          // purchaseEditTitle
    "Purchase",                                                   // purchaseDetailTitle
    "Products",                                                   // productsTitle
    "Add a product",                                              // newProductButton
    "Add a product",                                              // productNewTitle
    "Edit the product",                                           // productEditTitle
    "Shops",                                                      // shopsTitle
    "Add a shop",                                                 // newShopButton
    "Add a shop",                                                 // shopNewTitle
    "Edit the shop",                                              // shopEditTitle
    "Save",                                                       // saveButton
    "Cancel",                                                     // cancelButton
    "Edit",                                                       // editButton
    "Delete",                                                     // deleteButton
    "Select",                                                     // selectButton
    "Add",                                                        // addButton
    " / ",                                                        // rowSubtitleSeparator
    "No records",                                                 // noRecords
    "Not set",                                                    // unsetLabel
    "Saved.",                                                     // savedMessage
    "Shop name",                                                  // shopNameLabel
    "Product name",                                               // productNameLabel
    "Address",                                                    // addressLabel
    "Purchased on",                                               // purchasedOnLabel
    "Brewed at",                                                  // brewedAtLabel
    "Price",                                                      // priceLabel
    "Currency",                                                   // currencyLabel
    "UAE Dirham",                                                 // currencyAed
    "Australian Dollar",                                          // currencyAud
    "Brazilian Real",                                             // currencyBrl
    "Canadian Dollar",                                            // currencyCad
    "Swiss Franc",                                                // currencyChf
    "Chinese Yuan",                                               // currencyCny
    "Czech Koruna",                                               // currencyCzk
    "Danish Krone",                                               // currencyDkk
    "Euro",                                                       // currencyEur
    "British Pound",                                              // currencyGbp
    "Hong Kong Dollar",                                           // currencyHkd
    "Indonesian Rupiah",                                          // currencyIdr
    "Indian Rupee",                                               // currencyInr
    "Japanese Yen",                                               // currencyJpy
    "South Korean Won",                                           // currencyKrw
    "Mexican Peso",                                               // currencyMxn
    "Malaysian Ringgit",                                          // currencyMyr
    "Norwegian Krone",                                            // currencyNok
    "New Zealand Dollar",                                         // currencyNzd
    "Philippine Peso",                                            // currencyPhp
    "Polish Zloty",                                               // currencyPln
    "Saudi Riyal",                                                // currencySar
    "Swedish Krona",                                              // currencySek
    "Singapore Dollar",                                           // currencySgd
    "Thai Baht",                                                  // currencyThb
    "Turkish Lira",                                               // currencyTry
    "New Taiwan Dollar",                                          // currencyTwd
    "US Dollar",                                                  // currencyUsd
    "Vietnamese Dong",                                            // currencyVnd
    "South African Rand",                                         // currencyZar
    "Weight",                                                     // weightLabel
    "Dose",                                                       // doseLabel
    "Water",                                                      // waterLabel
    "Water temperature",                                          // waterTempLabel
    "Time",                                                       // brewTimeLabel
    "Method",                                                     // methodLabel
    "Grind setting",                                              // grindSettingLabel
    "Rating",                                                     // ratingLabel
    "Notes",                                                      // notesLabel
    "Product",                                                    // productLabel
    "Shop",                                                       // shopLabel
    "Purchase",                                                   // purchaseLabel
    "Enter a tag",                                                // tagInputHint
    "Tag name",                                                   // tagInputLabel
    "YYYY-MM-DD",                                                 // dayFormatHint
    "HH:MM",                                                      // timeFormatHint
    "Photo",                                                      // photoLabel
    "Choose a photo",                                             // photoSelectButton
    "Replace the photo",                                          // photoReplaceButton
    "Delete the photo",                                           // photoDeleteButton
    "No photo",                                                   // photoNoneLabel
    "Rating history",                                             // ratingHistoryTitle
    "Choose a purchase",                                          // selectPurchaseTitle
    "Choose a product",                                           // selectProductTitle
    "Choose a shop",                                              // selectShopTitle
    "No shop",                                                    // shopNoneLabel
    "This field is required.",                                    // validationRequired
    "Enter an integer of 0 or more.",                             // validationNumber
    "Enter a number of 0 or more with at most one decimal place.", // validationDecimal
    "Enter the date as YYYY-MM-DD.",                              // validationDay
    "Enter the time as HH:MM.",                                   // validationTime
    "Choose a product.",                                          // validationProduct
    "Choose a purchase.",                                         // validationPurchase
    "{date} / {shop}",                                            // brewRowSubtitle
    "{date}",                                                     // brewRowSubtitleNoShop
    "{value} g",                                                  // gramsValue
    "g",                                                          // gramUnit
    "°C",                                                         // celsiusUnit
    "s",                                                          // secondUnit
    "{value} °C",                                                 // celsiusValue
    "{value} s",                                                  // secondsValue
    "{value} / 5",                                                // ratingValue
    "{amount} {currency}",                                        // priceValue
    "Stats",                                                      // statsTitle
    "This month",                                                 // statsPeriodCurrentMonth
    "3 months",                                                   // statsPeriodThreeMonths
    "6 months",                                                   // statsPeriodSixMonths
    "12 months",                                                  // statsPeriodTwelveMonths
    "All time",                                                   // statsPeriodAllTime
    "Custom",                                                     // statsPeriodCustom
    "Start date",                                                 // statsStartLabel
    "End date",                                                   // statsEndLabel
    "Apply",                                                      // statsApplyButton
    "Enter a date on or after the start date.",                   // validationPeriod
    "Brew count",                                                 // statsBrewCountTitle
    "Dose",                                                       // statsBrewDoseTitle
    "Purchase amount ({currency})",                               // statsPurchaseAmountTitle
    "Purchase weight ({currency})",                               // statsPurchaseWeightTitle
    "No currency",                                                // statsCurrencyNone
    "Dose and rating",                                            // statsRatingDoseTitle
    "Water and rating",                                           // statsRatingWaterTitle
    "Water temperature and rating",                               // statsRatingTempTitle
    "Time and rating",                                            // statsRatingTimeTitle
    "Settings",                                                   // settingsTitle
    "Passkeys",                                                   // passkeysTitle
    "Add or remove the passkeys you use to log in.",              // passkeysDescription
    "Add a passkey",                                              // addPasskeyTitle
    "Add a passkey",                                              // addPasskeyButton
    "Rename the passkey",                                         // renamePasskeyTitle
    "Rename",                                                     // renameButton
    "Created: {timestamp}",                                       // passkeyCreatedAt
    "Last used: {timestamp}",                                     // passkeyLastUsedAt
    "Not used yet",                                               // passkeyNotUsedYet
    "Added the passkey.",                                         // passkeyAddedMessage
    "Renamed the passkey.",                                       // passkeyRenamedMessage
    "Deleted the passkey.",                                       // passkeyDeletedMessage
    "The last passkey cannot be deleted. Add another passkey first.", // passkeyLastDeleteError
    "Export",                                                     // exportTitle
    "Download all of your records as a single JSON file.",        // exportDescription
    "Download the export",                                        // exportButton
    "Downloaded the export.",                                     // exportDoneMessage
    "Delete the account",                                         // deleteAccountTitle
    "Deletes your account and all of your records. This cannot be undone.", // deleteAccountDescription
    "Delete the account",                                                   // deleteAccountButton
    "Delete",                                                               // deleteConfirmButton
    "Delete the account?", // deleteAccountConfirmTitle
    "Your account and all of your records will be deleted. This cannot be undone.", // deleteAccountConfirmMessage
    "Required",                                                                     // requiredLabel
    "Brews",                                                                        // brewsLabel
    "Beans used",                                                       // usedBeansLabel
    "No address",                                                       // addressUnset
    "Not rated",                                                        // ratingNone
    "Use the button at the bottom right to log your first cup.",        // homeEmptyHint
    "Use the button at the bottom right to log your first purchase.",   // purchasesEmptyHint
    "Use the button at the bottom right to add your first product.",    // productsEmptyHint
    "Use the button at the bottom right to add your first shop.",       // shopsEmptyHint
    "Choose a purchase",                                                // purchasePickPlaceholder
    "e.g. Comandante 24",                                               // grindSettingHint
    "Converts to JPEG and saves at a longest side of 2048 px or less.", // photoConvertNote
    "Uploading",                                                        // uploadingLabel
    "Reading the photo",                                                // suggestionLoadingLabel
    "Could not read the photo. You can continue by entering the values manually.", // suggestionFailedMessage
    "Add a product with the suggested values", // suggestionRegisterProductButton
    "If you cannot log in, ask the administrator for a registration link.", // loginRegisterHint
    "Contact the administrator to have a new registration link issued.", // registerTokenGuidance
    "From ",                                   // statsRangePrefix
    " to ",                                    // statsRangeMiddle
    ", {granularity}",                         // statsRangeSuffix
    "All time, {granularity}",                 // statsRangeAllTime
    "daily",                                   // statsGranularityDaily
    "monthly",                                 // statsGranularityMonthly
    "{days} days, so showing {granularity} (daily if 62 days or fewer).", // statsCustomGranularityNote
    "{unit} / {period}",                                                  // statsChartUnit
    "cups",                                                               // statsUnitCups
    "day",                                                                // statsPeriodDay
    "month",                                                              // statsPeriodMonth
    "Brew conditions and rating",                                         // statsScatterSection
    "Purchase amount", // statsPurchaseAmountLabel
    "Purchase weight", // statsPurchaseWeightLabel
];
