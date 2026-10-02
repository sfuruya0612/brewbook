//! 日本語の表 (FR-16)。
//!
//! `frontend/lib/l10n/app_ja.arb` から写す。`JA[key.index()]` で引く。

use super::keys::KEY_COUNT;

/// キーの添字で引く日本語の文言。
pub(super) const JA: [&str; KEY_COUNT] = [
    "brewbook",                                                                    // appTitle
    "読み込み中",                                                                  // loading
    "ログイン",                                                                    // loginTitle
    "パスキーでログインします。利用者名とパスワードの入力は要りません。", // loginDescription
    "パスキーでログイン",                                                 // loginButton
    "ログインできませんでした。もう一度お試しください。",                 // loginFailed
    "ログアウト",                                                         // logoutButton
    "パスキーの登録",                                                     // registerTitle
    "パスキーに付ける名前を入力して登録します。",                         // registerDescription
    "パスキーの名前",                                                     // passkeyNameLabel
    "例: 自宅の PC",                                                      // passkeyNameHint
    "1 文字以上 50 文字以下で入力してください。",                         // passkeyNameHelper
    "名前を 1 文字以上 50 文字以下で入力してください。",                  // passkeyNameError
    "登録する",                                                           // registerButton
    "登録用のトークンが URL にありません。",                              // registerTokenMissing
    "登録用のリンクが正しくありません。",                                 // registerTokenNotFound
    "この登録用のリンクは使用済みです。",                                 // registerTokenUsed
    "この登録用のリンクの有効期限が切れています。",                       // registerTokenExpired
    "brewbook",                                                           // homeTitle
    "再試行",                                                             // retryButton
    "入力の内容を確認してください。",                                     // errorValidation
    "セッションが無効になりました。もう一度ログインしてください。",       // errorUnauthorized
    "対象が見つかりません。",                                             // errorNotFound
    "処理できない状態です。画面を開き直してください。",                   // errorConflict
    "有効期限が切れています。",                                           // errorGone
    "通信に失敗しました。接続を確認してもう一度お試しください。",         // errorNetwork
    "予期しないエラーが発生しました。",                                   // errorUnexpected
    "パスキーの操作が取り消されました。",                                 // passkeyCancelled
    "この環境ではパスキーを使えません。",                                 // passkeyUnsupported
    "生産者",                                                             // producer
    "生産国",                                                             // origin
    "地域",                                                               // region
    "精製方法",                                                           // process
    "品種",                                                               // variety
    "焙煎度",                                                             // roast
    "焙煎日",                                                             // roastDate
    "フレーバーノート",                                                   // flavorNotes
    "メニュー",                                                           // menuTooltip
    "抽出を記録",                                                         // newBrewButton
    "抽出を記録",                                                         // brewNewTitle
    "抽出を編集",                                                         // brewEditTitle
    "抽出の詳細",                                                         // brewDetailTitle
    "購入",                                                               // purchasesTitle
    "購入を記録",                                                         // newPurchaseButton
    "購入を記録",                                                         // purchaseNewTitle
    "購入を編集",                                                         // purchaseEditTitle
    "購入の詳細",                                                         // purchaseDetailTitle
    "商品",                                                               // productsTitle
    "商品を登録",                                                         // newProductButton
    "商品を登録",                                                         // productNewTitle
    "商品を編集",                                                         // productEditTitle
    "店",                                                                 // shopsTitle
    "店を登録",                                                           // newShopButton
    "店を登録",                                                           // shopNewTitle
    "店を編集",                                                           // shopEditTitle
    "保存",                                                               // saveButton
    "キャンセル",                                                         // cancelButton
    "編集",                                                               // editButton
    "削除",                                                               // deleteButton
    "アーカイブ",                                                         // archiveButton
    "アーカイブ解除",                                                     // unarchiveButton
    "選択",                                                               // selectButton
    "追加",                                                               // addButton
    "アーカイブ済みを含める",                                             // includeArchivedLabel
    " / ",                                                                // rowSubtitleSeparator
    "記録がありません",                                                   // noRecords
    "未設定",                                                             // unsetLabel
    "保存しました。",                                                     // savedMessage
    "アーカイブしました。",                                               // archivedMessage
    "アーカイブ解除しました。",                                           // unarchivedMessage
    "店名",                                                               // shopNameLabel
    "商品名",                                                             // productNameLabel
    "住所",                                                               // addressLabel
    "購入日",                                                             // purchasedOnLabel
    "抽出日時",                                                           // brewedAtLabel
    "価格",                                                               // priceLabel
    "通貨",                                                               // currencyLabel
    "UAE ディルハム",                                                     // currencyAed
    "豪ドル",                                                             // currencyAud
    "ブラジル レアル",                                                    // currencyBrl
    "カナダ ドル",                                                        // currencyCad
    "スイス フラン",                                                      // currencyChf
    "中国人民元",                                                         // currencyCny
    "チェコ コルナ",                                                      // currencyCzk
    "デンマーク クローネ",                                                // currencyDkk
    "ユーロ",                                                             // currencyEur
    "英ポンド",                                                           // currencyGbp
    "香港ドル",                                                           // currencyHkd
    "インドネシア ルピア",                                                // currencyIdr
    "インド ルピー",                                                      // currencyInr
    "日本円",                                                             // currencyJpy
    "韓国ウォン",                                                         // currencyKrw
    "メキシコ ペソ",                                                      // currencyMxn
    "マレーシア リンギット",                                              // currencyMyr
    "ノルウェー クローネ",                                                // currencyNok
    "ニュージーランド ドル",                                              // currencyNzd
    "フィリピン ペソ",                                                    // currencyPhp
    "ポーランド ズウォティ",                                              // currencyPln
    "サウジアラビア リヤル",                                              // currencySar
    "スウェーデン クローナ",                                              // currencySek
    "シンガポール ドル",                                                  // currencySgd
    "タイ バーツ",                                                        // currencyThb
    "トルコ リラ",                                                        // currencyTry
    "新台湾ドル",                                                         // currencyTwd
    "米ドル",                                                             // currencyUsd
    "ベトナム ドン",                                                      // currencyVnd
    "南アフリカ ランド",                                                  // currencyZar
    "重量",                                                               // weightLabel
    "豆の量",                                                             // doseLabel
    "湯量",                                                               // waterLabel
    "湯の温度",                                                           // waterTempLabel
    "時間",                                                               // brewTimeLabel
    "抽出方法",                                                           // methodLabel
    "挽き目",                                                             // grindSettingLabel
    "評価",                                                               // ratingLabel
    "感想",                                                               // notesLabel
    "商品",                                                               // productLabel
    "店",                                                                 // shopLabel
    "購入",                                                               // purchaseLabel
    "タグを入力",                                                         // tagInputHint
    "タグの名前",                                                         // tagInputLabel
    "YYYY-MM-DD",                                                         // dayFormatHint
    "HH:MM",                                                              // timeFormatHint
    "写真",                                                               // photoLabel
    "写真を選ぶ",                                                         // photoSelectButton
    "写真を差し替え",                                                     // photoReplaceButton
    "写真を削除",                                                         // photoDeleteButton
    "写真はありません",                                                   // photoNoneLabel
    "評価の推移",                                                         // ratingHistoryTitle
    "購入を選ぶ",                                                         // selectPurchaseTitle
    "商品を選ぶ",                                                         // selectProductTitle
    "店を選ぶ",                                                           // selectShopTitle
    "店を指定しない",                                                     // shopNoneLabel
    "必須の項目です。",                                                   // validationRequired
    "0 以上の整数を入力してください。",                                   // validationNumber
    "0 以上で小数第 1 位までの数を入力してください。",                    // validationDecimal
    "日付を YYYY-MM-DD で入力してください。",                             // validationDay
    "時刻を HH:MM で入力してください。",                                  // validationTime
    "商品を選んでください。",                                             // validationProduct
    "購入を選んでください。",                                             // validationPurchase
    "{date} / {shop}",                                                    // brewRowSubtitle
    "{date}",                                                             // brewRowSubtitleNoShop
    "{value} g",                                                          // gramsValue
    "g",                                                                  // gramUnit
    "℃",                                                                  // celsiusUnit
    "秒",                                                                 // secondUnit
    "{value} ℃",                                                          // celsiusValue
    "{value} 秒",                                                         // secondsValue
    "{value} / 5",                                                        // ratingValue
    "{amount} {currency}",                                                // priceValue
    "統計",                                                               // statsTitle
    "当月",                                                               // statsPeriodCurrentMonth
    "3 か月",                                                             // statsPeriodThreeMonths
    "6 か月",                                                             // statsPeriodSixMonths
    "12 か月",                                                            // statsPeriodTwelveMonths
    "全期間",                                                             // statsPeriodAllTime
    "任意",                                                               // statsPeriodCustom
    "開始日",                                                             // statsStartLabel
    "終了日",                                                             // statsEndLabel
    "適用",                                                               // statsApplyButton
    "開始日以降の日付を入力してください。",                               // validationPeriod
    "抽出回数",                                                           // statsBrewCountTitle
    "豆の消費量",                                                         // statsBrewDoseTitle
    "購入金額 ({currency})", // statsPurchaseAmountTitle
    "購入重量 ({currency})", // statsPurchaseWeightTitle
    "通貨なし",              // statsCurrencyNone
    "豆の量と評価",          // statsRatingDoseTitle
    "湯量と評価",            // statsRatingWaterTitle
    "湯の温度と評価",        // statsRatingTempTitle
    "時間と評価",            // statsRatingTimeTitle
    "設定",                  // settingsTitle
    "パスキー",              // passkeysTitle
    "ログインに使うパスキーを追加したり削除したりできます。", // passkeysDescription
    "パスキーを追加",        // addPasskeyTitle
    "パスキーを追加",        // addPasskeyButton
    "パスキーの名前を変更",  // renamePasskeyTitle
    "名前を変更",            // renameButton
    "登録: {timestamp}",     // passkeyCreatedAt
    "最終使用: {timestamp}", // passkeyLastUsedAt
    "まだ使われていません",  // passkeyNotUsedYet
    "パスキーを追加しました。", // passkeyAddedMessage
    "パスキーの名前を変更しました。", // passkeyRenamedMessage
    "パスキーを削除しました。", // passkeyDeletedMessage
    "最後の 1 つのパスキーは削除できません。先に別のパスキーを追加してください。", // passkeyLastDeleteError
    "エクスポート",                                                                // exportTitle
    "全記録を 1 つの JSON ファイルとしてダウンロードできます。", // exportDescription
    "エクスポートをダウンロード",                                // exportButton
    "エクスポートをダウンロードしました。",                      // exportDoneMessage
    "アカウントの削除",                                          // deleteAccountTitle
    "アカウントと全記録を削除します。元に戻せません。",          // deleteAccountDescription
    "アカウントを削除",                                          // deleteAccountButton
    "削除する",                                                  // deleteConfirmButton
    "アカウントを削除しますか？",                                // deleteAccountConfirmTitle
    "アカウントと全記録を削除します。元に戻せません。",          // deleteAccountConfirmMessage
    "必須",                                                      // requiredLabel
    "抽出",                                                      // brewsLabel
    "使った豆",                                                  // usedBeansLabel
    "アーカイブ済み",                                            // archivedBadge
    "住所は未設定",                                              // addressUnset
    "未評価",                                                    // ratingNone
    "右下の「抽出を記録」から最初の 1 杯を記録します。",         // homeEmptyHint
    "右下の「購入を記録」から最初の 1 件を記録します。",         // purchasesEmptyHint
    "右下の「商品を登録」から最初の 1 件を登録します。",         // productsEmptyHint
    "右下の「店を登録」から最初の 1 件を登録します。",           // shopsEmptyHint
    "購入を選ぶ",                                                // purchasePickPlaceholder
    "例: Comandante 24",                                         // grindSettingHint
    "JPEG に変換し、長辺 2048 px 以下に縮小して保存します。",    // photoConvertNote
    "アップロード中",                                            // uploadingLabel
    "写真から推測中",                                            // suggestionLoadingLabel
    "写真からの推測に失敗しました。手入力で続けられます。",      // suggestionFailedMessage
    "推測した内容で商品を登録",                                  // suggestionRegisterProductButton
    "ログインできないときは管理者から登録用のリンクを受け取ってください。", // loginRegisterHint
    "管理者に連絡して、登録用のリンクを再発行してもらってください。", // registerTokenGuidance
    "",                                                          // statsRangePrefix
    " から ",                                                    // statsRangeMiddle
    " まで、{granularity}",                                      // statsRangeSuffix
    "全期間、{granularity}",                                     // statsRangeAllTime
    "日別",                                                      // statsGranularityDaily
    "月別",                                                      // statsGranularityMonthly
    "{days} 日なので{granularity}で表示します (62 日以下なら日別)。", // statsCustomGranularityNote
    "{unit} / {period}",                                         // statsChartUnit
    "杯",                                                        // statsUnitCups
    "日",                                                        // statsPeriodDay
    "月",                                                        // statsPeriodMonth
    "抽出条件と評価",                                            // statsScatterSection
    "購入金額",                                                  // statsPurchaseAmountLabel
    "購入重量",                                                  // statsPurchaseWeightLabel
];
