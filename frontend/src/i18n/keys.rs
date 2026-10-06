//! 翻訳のキー (FR-16)。
//!
//! 移行前の Flutter 版の ARB (`frontend/lib/l10n/app_ja.arb` と `app_en.arb`。0045 で削除) の
//! 221 キーから作る (0051 で並び替えとお気に入りの 8 キー、0054 で抽出方法の例と破棄の確認の
//! 4 キーを足した)。表 ([`super::ja`] と [`super::en`]) はこの添字で引く。キーの順序は ARB と
//! 同じにしていた。

/// キーの数。
pub const KEY_COUNT: usize = 233;

/// 翻訳のキー (FR-16)。
///
/// 画面はこのキーで文言を引き、表示する文字列を直接書かない。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Key {
    /// `appTitle`。
    AppTitle,
    /// `loading`。
    Loading,
    /// `loginTitle`。
    LoginTitle,
    /// `loginDescription`。
    LoginDescription,
    /// `loginButton`。
    LoginButton,
    /// `loginFailed`。
    LoginFailed,
    /// `logoutButton`。
    LogoutButton,
    /// `registerTitle`。
    RegisterTitle,
    /// `registerDescription`。
    RegisterDescription,
    /// `passkeyNameLabel`。
    PasskeyNameLabel,
    /// `passkeyNameHint`。
    PasskeyNameHint,
    /// `passkeyNameHelper`。
    PasskeyNameHelper,
    /// `passkeyNameError`。
    PasskeyNameError,
    /// `registerButton`。
    RegisterButton,
    /// `registerTokenMissing`。
    RegisterTokenMissing,
    /// `registerTokenNotFound`。
    RegisterTokenNotFound,
    /// `registerTokenUsed`。
    RegisterTokenUsed,
    /// `registerTokenExpired`。
    RegisterTokenExpired,
    /// `homeTitle`。
    HomeTitle,
    /// `retryButton`。
    RetryButton,
    /// `errorValidation`。
    ErrorValidation,
    /// `errorUnauthorized`。
    ErrorUnauthorized,
    /// `errorNotFound`。
    ErrorNotFound,
    /// `errorConflict`。
    ErrorConflict,
    /// `errorGone`。
    ErrorGone,
    /// `errorNetwork`。
    ErrorNetwork,
    /// `errorUnexpected`。
    ErrorUnexpected,
    /// `passkeyCancelled`。
    PasskeyCancelled,
    /// `passkeyUnsupported`。
    PasskeyUnsupported,
    /// `producer`。
    Producer,
    /// `origin`。
    Origin,
    /// `region`。
    Region,
    /// `process`。
    Process,
    /// `variety`。
    Variety,
    /// `roast`。
    Roast,
    /// `roastDate`。
    RoastDate,
    /// `flavorNotes`。
    FlavorNotes,
    /// `menuTooltip`。
    MenuTooltip,
    /// `newBrewButton`。
    NewBrewButton,
    /// `brewNewTitle`。
    BrewNewTitle,
    /// `brewEditTitle`。
    BrewEditTitle,
    /// `brewDetailTitle`。
    BrewDetailTitle,
    /// `purchasesTitle`。
    PurchasesTitle,
    /// `newPurchaseButton`。
    NewPurchaseButton,
    /// `purchaseNewTitle`。
    PurchaseNewTitle,
    /// `purchaseEditTitle`。
    PurchaseEditTitle,
    /// `purchaseDetailTitle`。
    PurchaseDetailTitle,
    /// `productsTitle`。
    ProductsTitle,
    /// `newProductButton`。
    NewProductButton,
    /// `productNewTitle`。
    ProductNewTitle,
    /// `productEditTitle`。
    ProductEditTitle,
    /// `shopsTitle`。
    ShopsTitle,
    /// `newShopButton`。
    NewShopButton,
    /// `shopNewTitle`。
    ShopNewTitle,
    /// `shopEditTitle`。
    ShopEditTitle,
    /// `saveButton`。
    SaveButton,
    /// `cancelButton`。
    CancelButton,
    /// `editButton`。
    EditButton,
    /// `deleteButton`。
    DeleteButton,
    /// `selectButton`。
    SelectButton,
    /// `addButton`。
    AddButton,
    /// `rowSubtitleSeparator`。
    RowSubtitleSeparator,
    /// `noRecords`。
    NoRecords,
    /// `unsetLabel`。
    UnsetLabel,
    /// `savedMessage`。
    SavedMessage,
    /// `shopNameLabel`。
    ShopNameLabel,
    /// `productNameLabel`。
    ProductNameLabel,
    /// `addressLabel`。
    AddressLabel,
    /// `purchasedOnLabel`。
    PurchasedOnLabel,
    /// `brewedAtLabel`。
    BrewedAtLabel,
    /// `priceLabel`。
    PriceLabel,
    /// `currencyLabel`。
    CurrencyLabel,
    /// `currencyAed`。
    CurrencyAed,
    /// `currencyAud`。
    CurrencyAud,
    /// `currencyBrl`。
    CurrencyBrl,
    /// `currencyCad`。
    CurrencyCad,
    /// `currencyChf`。
    CurrencyChf,
    /// `currencyCny`。
    CurrencyCny,
    /// `currencyCzk`。
    CurrencyCzk,
    /// `currencyDkk`。
    CurrencyDkk,
    /// `currencyEur`。
    CurrencyEur,
    /// `currencyGbp`。
    CurrencyGbp,
    /// `currencyHkd`。
    CurrencyHkd,
    /// `currencyIdr`。
    CurrencyIdr,
    /// `currencyInr`。
    CurrencyInr,
    /// `currencyJpy`。
    CurrencyJpy,
    /// `currencyKrw`。
    CurrencyKrw,
    /// `currencyMxn`。
    CurrencyMxn,
    /// `currencyMyr`。
    CurrencyMyr,
    /// `currencyNok`。
    CurrencyNok,
    /// `currencyNzd`。
    CurrencyNzd,
    /// `currencyPhp`。
    CurrencyPhp,
    /// `currencyPln`。
    CurrencyPln,
    /// `currencySar`。
    CurrencySar,
    /// `currencySek`。
    CurrencySek,
    /// `currencySgd`。
    CurrencySgd,
    /// `currencyThb`。
    CurrencyThb,
    /// `currencyTry`。
    CurrencyTry,
    /// `currencyTwd`。
    CurrencyTwd,
    /// `currencyUsd`。
    CurrencyUsd,
    /// `currencyVnd`。
    CurrencyVnd,
    /// `currencyZar`。
    CurrencyZar,
    /// `weightLabel`。
    WeightLabel,
    /// `doseLabel`。
    DoseLabel,
    /// `waterLabel`。
    WaterLabel,
    /// `waterTempLabel`。
    WaterTempLabel,
    /// `brewTimeLabel`。
    BrewTimeLabel,
    /// `methodLabel`。
    MethodLabel,
    /// `grindSettingLabel`。
    GrindSettingLabel,
    /// `ratingLabel`。
    RatingLabel,
    /// `notesLabel`。
    NotesLabel,
    /// `productLabel`。
    ProductLabel,
    /// `shopLabel`。
    ShopLabel,
    /// `purchaseLabel`。
    PurchaseLabel,
    /// `tagInputHint`。
    TagInputHint,
    /// `tagInputLabel`。
    TagInputLabel,
    /// `dayFormatHint`。
    DayFormatHint,
    /// `timeFormatHint`。
    TimeFormatHint,
    /// `photoLabel`。
    PhotoLabel,
    /// `photoSelectButton`。
    PhotoSelectButton,
    /// `photoReplaceButton`。
    PhotoReplaceButton,
    /// `photoDeleteButton`。
    PhotoDeleteButton,
    /// `photoNoneLabel`。
    PhotoNoneLabel,
    /// `ratingHistoryTitle`。
    RatingHistoryTitle,
    /// `selectPurchaseTitle`。
    SelectPurchaseTitle,
    /// `selectProductTitle`。
    SelectProductTitle,
    /// `selectShopTitle`。
    SelectShopTitle,
    /// `shopNoneLabel`。
    ShopNoneLabel,
    /// `validationRequired`。
    ValidationRequired,
    /// `validationNumber`。
    ValidationNumber,
    /// `validationDecimal`。
    ValidationDecimal,
    /// `validationDay`。
    ValidationDay,
    /// `validationTime`。
    ValidationTime,
    /// `validationProduct`。
    ValidationProduct,
    /// `validationPurchase`。
    ValidationPurchase,
    /// `brewRowSubtitle`。
    BrewRowSubtitle,
    /// `brewRowSubtitleNoShop`。
    BrewRowSubtitleNoShop,
    /// `gramsValue`。
    GramsValue,
    /// `gramUnit`。
    GramUnit,
    /// `celsiusUnit`。
    CelsiusUnit,
    /// `secondUnit`。
    SecondUnit,
    /// `celsiusValue`。
    CelsiusValue,
    /// `secondsValue`。
    SecondsValue,
    /// `ratingValue`。
    RatingValue,
    /// `priceValue`。
    PriceValue,
    /// `statsTitle`。
    StatsTitle,
    /// `statsPeriodCurrentMonth`。
    StatsPeriodCurrentMonth,
    /// `statsPeriodThreeMonths`。
    StatsPeriodThreeMonths,
    /// `statsPeriodSixMonths`。
    StatsPeriodSixMonths,
    /// `statsPeriodTwelveMonths`。
    StatsPeriodTwelveMonths,
    /// `statsPeriodAllTime`。
    StatsPeriodAllTime,
    /// `statsPeriodCustom`。
    StatsPeriodCustom,
    /// `statsStartLabel`。
    StatsStartLabel,
    /// `statsEndLabel`。
    StatsEndLabel,
    /// `statsApplyButton`。
    StatsApplyButton,
    /// `validationPeriod`。
    ValidationPeriod,
    /// `statsBrewCountTitle`。
    StatsBrewCountTitle,
    /// `statsBrewDoseTitle`。
    StatsBrewDoseTitle,
    /// `statsPurchaseAmountTitle`。
    StatsPurchaseAmountTitle,
    /// `statsPurchaseWeightTitle`。
    StatsPurchaseWeightTitle,
    /// `statsCurrencyNone`。
    StatsCurrencyNone,
    /// `statsRatingDoseTitle`。
    StatsRatingDoseTitle,
    /// `statsRatingWaterTitle`。
    StatsRatingWaterTitle,
    /// `statsRatingTempTitle`。
    StatsRatingTempTitle,
    /// `statsRatingTimeTitle`。
    StatsRatingTimeTitle,
    /// `settingsTitle`。
    SettingsTitle,
    /// `passkeysTitle`。
    PasskeysTitle,
    /// `passkeysDescription`。
    PasskeysDescription,
    /// `addPasskeyTitle`。
    AddPasskeyTitle,
    /// `addPasskeyButton`。
    AddPasskeyButton,
    /// `renamePasskeyTitle`。
    RenamePasskeyTitle,
    /// `renameButton`。
    RenameButton,
    /// `passkeyCreatedAt`。
    PasskeyCreatedAt,
    /// `passkeyLastUsedAt`。
    PasskeyLastUsedAt,
    /// `passkeyNotUsedYet`。
    PasskeyNotUsedYet,
    /// `passkeyAddedMessage`。
    PasskeyAddedMessage,
    /// `passkeyRenamedMessage`。
    PasskeyRenamedMessage,
    /// `passkeyDeletedMessage`。
    PasskeyDeletedMessage,
    /// `passkeyLastDeleteError`。
    PasskeyLastDeleteError,
    /// `exportTitle`。
    ExportTitle,
    /// `exportDescription`。
    ExportDescription,
    /// `exportButton`。
    ExportButton,
    /// `exportDoneMessage`。
    ExportDoneMessage,
    /// `deleteAccountTitle`。
    DeleteAccountTitle,
    /// `deleteAccountDescription`。
    DeleteAccountDescription,
    /// `deleteAccountButton`。
    DeleteAccountButton,
    /// `deleteConfirmButton`。
    DeleteConfirmButton,
    /// `deleteAccountConfirmTitle`。
    DeleteAccountConfirmTitle,
    /// `deleteAccountConfirmMessage`。
    DeleteAccountConfirmMessage,
    /// `requiredLabel`。
    RequiredLabel,
    /// `brewsLabel`。
    BrewsLabel,
    /// `usedBeansLabel`。
    UsedBeansLabel,
    /// `addressUnset`。
    AddressUnset,
    /// `ratingNone`。
    RatingNone,
    /// `homeEmptyHint`。
    HomeEmptyHint,
    /// `purchasesEmptyHint`。
    PurchasesEmptyHint,
    /// `productsEmptyHint`。
    ProductsEmptyHint,
    /// `shopsEmptyHint`。
    ShopsEmptyHint,
    /// `purchasePickPlaceholder`。
    PurchasePickPlaceholder,
    /// `grindSettingHint`。
    GrindSettingHint,
    /// `photoConvertNote`。
    PhotoConvertNote,
    /// `uploadingLabel`。
    UploadingLabel,
    /// `suggestionLoadingLabel`。
    SuggestionLoadingLabel,
    /// `suggestionFailedMessage`。
    SuggestionFailedMessage,
    /// `suggestionRegisterProductButton`。
    SuggestionRegisterProductButton,
    /// `loginRegisterHint`。
    LoginRegisterHint,
    /// `registerTokenGuidance`。
    RegisterTokenGuidance,
    /// `statsRangePrefix`。
    StatsRangePrefix,
    /// `statsRangeMiddle`。
    StatsRangeMiddle,
    /// `statsRangeSuffix`。
    StatsRangeSuffix,
    /// `statsRangeAllTime`。
    StatsRangeAllTime,
    /// `statsGranularityDaily`。
    StatsGranularityDaily,
    /// `statsGranularityMonthly`。
    StatsGranularityMonthly,
    /// `statsCustomGranularityNote`。
    StatsCustomGranularityNote,
    /// `statsChartUnit`。
    StatsChartUnit,
    /// `statsUnitCups`。
    StatsUnitCups,
    /// `statsPeriodDay`。
    StatsPeriodDay,
    /// `statsPeriodMonth`。
    StatsPeriodMonth,
    /// `statsScatterSection`。
    StatsScatterSection,
    /// `statsPurchaseAmountLabel`。
    StatsPurchaseAmountLabel,
    /// `statsPurchaseWeightLabel`。
    StatsPurchaseWeightLabel,
    /// `sortLabel`。
    SortLabel,
    /// `sortAscending`。
    SortAscending,
    /// `sortDescending`。
    SortDescending,
    /// `favoritesOnlyLabel`。
    FavoritesOnlyLabel,
    /// `favoriteAddLabel`。
    FavoriteAddLabel,
    /// `favoriteRemoveLabel`。
    FavoriteRemoveLabel,
    /// `sortCreatedAt`。
    SortCreatedAt,
    /// `sortUpdatedAt`。
    SortUpdatedAt,
    /// `methodHint`。
    MethodHint,
    /// `discardConfirmTitle`。
    DiscardConfirmTitle,
    /// `discardConfirmMessage`。
    DiscardConfirmMessage,
    /// `discardConfirmButton`。
    DiscardConfirmButton,
}

impl Key {
    /// 全てのキー (ARB と同じ順序)。
    pub const ALL: [Key; KEY_COUNT] = [
        Key::AppTitle,
        Key::Loading,
        Key::LoginTitle,
        Key::LoginDescription,
        Key::LoginButton,
        Key::LoginFailed,
        Key::LogoutButton,
        Key::RegisterTitle,
        Key::RegisterDescription,
        Key::PasskeyNameLabel,
        Key::PasskeyNameHint,
        Key::PasskeyNameHelper,
        Key::PasskeyNameError,
        Key::RegisterButton,
        Key::RegisterTokenMissing,
        Key::RegisterTokenNotFound,
        Key::RegisterTokenUsed,
        Key::RegisterTokenExpired,
        Key::HomeTitle,
        Key::RetryButton,
        Key::ErrorValidation,
        Key::ErrorUnauthorized,
        Key::ErrorNotFound,
        Key::ErrorConflict,
        Key::ErrorGone,
        Key::ErrorNetwork,
        Key::ErrorUnexpected,
        Key::PasskeyCancelled,
        Key::PasskeyUnsupported,
        Key::Producer,
        Key::Origin,
        Key::Region,
        Key::Process,
        Key::Variety,
        Key::Roast,
        Key::RoastDate,
        Key::FlavorNotes,
        Key::MenuTooltip,
        Key::NewBrewButton,
        Key::BrewNewTitle,
        Key::BrewEditTitle,
        Key::BrewDetailTitle,
        Key::PurchasesTitle,
        Key::NewPurchaseButton,
        Key::PurchaseNewTitle,
        Key::PurchaseEditTitle,
        Key::PurchaseDetailTitle,
        Key::ProductsTitle,
        Key::NewProductButton,
        Key::ProductNewTitle,
        Key::ProductEditTitle,
        Key::ShopsTitle,
        Key::NewShopButton,
        Key::ShopNewTitle,
        Key::ShopEditTitle,
        Key::SaveButton,
        Key::CancelButton,
        Key::EditButton,
        Key::DeleteButton,
        Key::SelectButton,
        Key::AddButton,
        Key::RowSubtitleSeparator,
        Key::NoRecords,
        Key::UnsetLabel,
        Key::SavedMessage,
        Key::ShopNameLabel,
        Key::ProductNameLabel,
        Key::AddressLabel,
        Key::PurchasedOnLabel,
        Key::BrewedAtLabel,
        Key::PriceLabel,
        Key::CurrencyLabel,
        Key::CurrencyAed,
        Key::CurrencyAud,
        Key::CurrencyBrl,
        Key::CurrencyCad,
        Key::CurrencyChf,
        Key::CurrencyCny,
        Key::CurrencyCzk,
        Key::CurrencyDkk,
        Key::CurrencyEur,
        Key::CurrencyGbp,
        Key::CurrencyHkd,
        Key::CurrencyIdr,
        Key::CurrencyInr,
        Key::CurrencyJpy,
        Key::CurrencyKrw,
        Key::CurrencyMxn,
        Key::CurrencyMyr,
        Key::CurrencyNok,
        Key::CurrencyNzd,
        Key::CurrencyPhp,
        Key::CurrencyPln,
        Key::CurrencySar,
        Key::CurrencySek,
        Key::CurrencySgd,
        Key::CurrencyThb,
        Key::CurrencyTry,
        Key::CurrencyTwd,
        Key::CurrencyUsd,
        Key::CurrencyVnd,
        Key::CurrencyZar,
        Key::WeightLabel,
        Key::DoseLabel,
        Key::WaterLabel,
        Key::WaterTempLabel,
        Key::BrewTimeLabel,
        Key::MethodLabel,
        Key::GrindSettingLabel,
        Key::RatingLabel,
        Key::NotesLabel,
        Key::ProductLabel,
        Key::ShopLabel,
        Key::PurchaseLabel,
        Key::TagInputHint,
        Key::TagInputLabel,
        Key::DayFormatHint,
        Key::TimeFormatHint,
        Key::PhotoLabel,
        Key::PhotoSelectButton,
        Key::PhotoReplaceButton,
        Key::PhotoDeleteButton,
        Key::PhotoNoneLabel,
        Key::RatingHistoryTitle,
        Key::SelectPurchaseTitle,
        Key::SelectProductTitle,
        Key::SelectShopTitle,
        Key::ShopNoneLabel,
        Key::ValidationRequired,
        Key::ValidationNumber,
        Key::ValidationDecimal,
        Key::ValidationDay,
        Key::ValidationTime,
        Key::ValidationProduct,
        Key::ValidationPurchase,
        Key::BrewRowSubtitle,
        Key::BrewRowSubtitleNoShop,
        Key::GramsValue,
        Key::GramUnit,
        Key::CelsiusUnit,
        Key::SecondUnit,
        Key::CelsiusValue,
        Key::SecondsValue,
        Key::RatingValue,
        Key::PriceValue,
        Key::StatsTitle,
        Key::StatsPeriodCurrentMonth,
        Key::StatsPeriodThreeMonths,
        Key::StatsPeriodSixMonths,
        Key::StatsPeriodTwelveMonths,
        Key::StatsPeriodAllTime,
        Key::StatsPeriodCustom,
        Key::StatsStartLabel,
        Key::StatsEndLabel,
        Key::StatsApplyButton,
        Key::ValidationPeriod,
        Key::StatsBrewCountTitle,
        Key::StatsBrewDoseTitle,
        Key::StatsPurchaseAmountTitle,
        Key::StatsPurchaseWeightTitle,
        Key::StatsCurrencyNone,
        Key::StatsRatingDoseTitle,
        Key::StatsRatingWaterTitle,
        Key::StatsRatingTempTitle,
        Key::StatsRatingTimeTitle,
        Key::SettingsTitle,
        Key::PasskeysTitle,
        Key::PasskeysDescription,
        Key::AddPasskeyTitle,
        Key::AddPasskeyButton,
        Key::RenamePasskeyTitle,
        Key::RenameButton,
        Key::PasskeyCreatedAt,
        Key::PasskeyLastUsedAt,
        Key::PasskeyNotUsedYet,
        Key::PasskeyAddedMessage,
        Key::PasskeyRenamedMessage,
        Key::PasskeyDeletedMessage,
        Key::PasskeyLastDeleteError,
        Key::ExportTitle,
        Key::ExportDescription,
        Key::ExportButton,
        Key::ExportDoneMessage,
        Key::DeleteAccountTitle,
        Key::DeleteAccountDescription,
        Key::DeleteAccountButton,
        Key::DeleteConfirmButton,
        Key::DeleteAccountConfirmTitle,
        Key::DeleteAccountConfirmMessage,
        Key::RequiredLabel,
        Key::BrewsLabel,
        Key::UsedBeansLabel,
        Key::AddressUnset,
        Key::RatingNone,
        Key::HomeEmptyHint,
        Key::PurchasesEmptyHint,
        Key::ProductsEmptyHint,
        Key::ShopsEmptyHint,
        Key::PurchasePickPlaceholder,
        Key::GrindSettingHint,
        Key::PhotoConvertNote,
        Key::UploadingLabel,
        Key::SuggestionLoadingLabel,
        Key::SuggestionFailedMessage,
        Key::SuggestionRegisterProductButton,
        Key::LoginRegisterHint,
        Key::RegisterTokenGuidance,
        Key::StatsRangePrefix,
        Key::StatsRangeMiddle,
        Key::StatsRangeSuffix,
        Key::StatsRangeAllTime,
        Key::StatsGranularityDaily,
        Key::StatsGranularityMonthly,
        Key::StatsCustomGranularityNote,
        Key::StatsChartUnit,
        Key::StatsUnitCups,
        Key::StatsPeriodDay,
        Key::StatsPeriodMonth,
        Key::StatsScatterSection,
        Key::StatsPurchaseAmountLabel,
        Key::StatsPurchaseWeightLabel,
        Key::SortLabel,
        Key::SortAscending,
        Key::SortDescending,
        Key::FavoritesOnlyLabel,
        Key::FavoriteAddLabel,
        Key::FavoriteRemoveLabel,
        Key::SortCreatedAt,
        Key::SortUpdatedAt,
        Key::MethodHint,
        Key::DiscardConfirmTitle,
        Key::DiscardConfirmMessage,
        Key::DiscardConfirmButton,
    ];

    /// 表の添字。
    pub const fn index(self) -> usize {
        self as usize
    }
}
