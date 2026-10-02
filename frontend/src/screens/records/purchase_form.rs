//! 購入の登録と編集の画面 (FR-9、FR-10、FR-19)。
//!
//! 商品は必須で、店は省略できる。購入日は端末のタイムゾーンでの当日を既定値にする (FR-9)。
//! 写真は選択した時点では変換と推測だけを行い、購入を保存した後にアップロードする
//! (FR-10、ADR-0003)。推測は空の入力欄にだけ入れ、一致する商品が無いときは登録の導線を出す
//! (FR-19)。

use dioxus::prelude::*;
use dioxus_router::navigator;

use crate::i18n::{current_language, t, Key};
use crate::records::values::format_day;
use crate::records::{
    apply_purchase_suggestion, currency_option_label, currency_options,
    product_match as decide_product_match, record_error_key, save_target, suggested_product_name,
    validate_purchase_form, ConvertedImage, Product, ProductMatch, ProductSuggestion,
    PurchaseFormErrors, PurchaseFormValues, RecordError, RecordServices, RecordsApi, SaveTarget,
    Shop, DEFAULT_CURRENCY, MAX_PHOTO_BYTES, MAX_PHOTO_LONG_SIDE,
};
use crate::ui::{AppBar, Banner, Button, ButtonVariant, Field, Icon, ListRow, TextField};

use super::{
    clear_notice_after, mark_records_changed, photo_preview_url, retryable_banner, RecordLoader,
    RecordPickerSheet,
};

/// 購入の登録 (FR-9)。
#[component]
pub fn PurchaseFormScreen() -> Element {
    rsx! {
        PurchaseForm {}
    }
}

/// 購入の編集 (FR-9)。
#[component]
pub fn PurchaseEditScreen(id: String) -> Element {
    rsx! {
        PurchaseForm { id: Some(id) }
    }
}

/// 購入の登録と編集のフォーム。2 段組の右の面にも出せる。
#[component]
pub fn PurchaseForm(
    /// 編集する購入の ID。新規の登録のときは None。
    #[props(default)]
    id: Option<String>,

    /// 2 段組の右の面に出すか。
    #[props(default = false)]
    embedded: bool,

    /// 閉じる動き。無いときは前の画面へ戻る。
    #[props(default)]
    on_close: Option<EventHandler<()>>,

    /// 保存できたときの動き。無いときは前の画面へ戻る。
    #[props(default)]
    on_saved: Option<EventHandler<()>>,
) -> Element {
    let services = use_context::<RecordServices>();
    let mut revision = use_context::<Signal<u64>>();
    let mut notice = use_context::<Signal<Option<String>>>();
    let navigator = navigator();

    // 購入日は端末のタイムゾーンでの当日を既定値にする (FR-9)。
    let today = services.clock.now().date();
    let mut product = use_signal(|| None::<Product>);
    let mut shop = use_signal(|| None::<Shop>);
    let mut purchased_on = use_signal(|| format_day(today));
    let mut roast = use_signal(String::new);
    let mut roast_date = use_signal(String::new);
    let mut price = use_signal(String::new);
    let mut currency = use_signal(|| DEFAULT_CURRENCY.to_string());
    let mut weight = use_signal(String::new);
    let mut picked = use_signal(|| None::<ConvertedImage>);
    let mut photo_key = use_signal(|| None::<String>);
    let mut remove_photo = use_signal(|| false);
    let mut photo_error = use_signal(|| None::<RecordError>);
    let mut errors = use_signal(PurchaseFormErrors::default);
    let mut save_error = use_signal(|| None::<RecordError>);
    let mut load_error = use_signal(|| None::<RecordError>);
    let mut loading = use_signal(|| id.is_some());
    let mut busy = use_signal(|| false);
    let mut picker = use_signal(|| None::<PickerKind>);
    let mut suggesting = use_signal(|| false);
    let mut suggestion_failed = use_signal(|| false);
    let mut unmatched_product = use_signal(|| None::<ProductSuggestion>);
    let mut suggestion_generation = use_signal(|| 0_u64);
    let mut created_id = use_signal(|| None::<String>);
    // 推測した内容で商品を登録する導線の入力 (FR-19)。
    let mut register_open = use_signal(|| false);
    let mut draft_name = use_signal(String::new);
    let mut draft_producer = use_signal(String::new);
    let mut draft_origin = use_signal(String::new);
    let mut draft_region = use_signal(String::new);
    let mut draft_process = use_signal(String::new);
    let mut draft_variety = use_signal(String::new);
    let mut draft_tags = use_signal(Vec::<String>::new);
    let mut draft_error = use_signal(|| None::<RecordError>);

    // 編集のために現在の値を読み込む。再試行でも同じ処理を呼ぶ。
    let reload_services = services.clone();
    let reload_id = id.clone();
    let reload = EventHandler::new(move |_| {
        let Some(id) = reload_id.clone() else {
            return;
        };
        let api = RecordsApi::new(reload_services.api.clone());
        spawn(async move {
            loading.set(true);
            load_error.set(None);
            match api.purchase(&id).await {
                Ok(purchase) => {
                    product.set(Some(purchase.product.clone()));
                    shop.set(purchase.shop.clone());
                    purchased_on.set(purchase.purchased_on.clone());
                    roast.set(purchase.roast.clone().unwrap_or_default());
                    roast_date.set(purchase.roast_date.clone().unwrap_or_default());
                    price.set(
                        purchase
                            .price_amount
                            .map(|amount| amount.to_string())
                            .unwrap_or_default(),
                    );
                    currency.set(
                        purchase
                            .price_currency
                            .clone()
                            .unwrap_or_else(|| DEFAULT_CURRENCY.to_string()),
                    );
                    weight.set(
                        purchase
                            .weight_grams
                            .map(|value| value.to_string())
                            .unwrap_or_default(),
                    );
                    photo_key.set(purchase.photo_key.clone());
                }
                Err(failure) => load_error.set(Some(failure)),
            }
            loading.set(false);
        });
    });
    use_effect(move || reload.call(()));

    // 写真を選び、JPEG に変換して長辺を縮める (FR-10)。変換の直後に推測を呼ぶ (FR-19)。
    let photo_services = services.clone();
    let suggest = EventHandler::new(move |image: ConvertedImage| {
        // 写真を選び直した時点で世代を進め、飛んでいる古い応答を無効にする (FR-19)。
        let generation = suggestion_generation() + 1;
        suggestion_generation.set(generation);
        unmatched_product.set(None);
        // 5 MB を超える写真では推測を呼ばない (FR-19)。
        if image.size() > MAX_PHOTO_BYTES {
            suggesting.set(false);
            suggestion_failed.set(false);
            return;
        }
        suggesting.set(true);
        suggestion_failed.set(false);
        let api = RecordsApi::new(photo_services.api.clone());
        spawn(async move {
            let suggestion = match api.suggest_purchase(&image.bytes).await {
                Ok(suggestion) => suggestion,
                Err(_) => {
                    if suggestion_generation() == generation {
                        suggestion_failed.set(true);
                        suggesting.set(false);
                    }
                    return;
                }
            };
            if suggestion_generation() != generation {
                return;
            }
            // 推測の反映は、商品の照合の成否と独立に行う (照合の失敗で推測を捨てない)。
            let mut roast_value = roast();
            let mut roast_date_value = roast_date();
            let mut price_value = price();
            let mut weight_value = weight();
            apply_purchase_suggestion(
                &mut roast_value,
                &mut roast_date_value,
                &mut price_value,
                &mut weight_value,
                &suggestion,
            );
            roast.set(roast_value);
            roast_date.set(roast_date_value);
            price.set(price_value);
            weight.set(weight_value);

            // 商品が未選択のときだけ、推測した商品名に一致する商品を探す (FR-19)。
            let name = suggestion.product.as_ref().and_then(suggested_product_name);
            if product().is_some() || name.is_none() {
                suggesting.set(false);
                unmatched_product.set(None);
                return;
            }
            let matched = match api.products(None, false, name.as_deref()).await {
                Ok(page) => page.items.into_iter().next(),
                Err(_) => {
                    if suggestion_generation() == generation {
                        suggesting.set(false);
                        unmatched_product.set(None);
                    }
                    return;
                }
            };
            if suggestion_generation() != generation {
                return;
            }
            match decide_product_match(product().is_some(), &suggestion, matched) {
                ProductMatch::Select(matched) => {
                    product.set(Some(matched));
                    unmatched_product.set(None);
                }
                ProductMatch::Register(suggested) => unmatched_product.set(Some(suggested)),
                ProductMatch::None => unmatched_product.set(None),
            }
            suggesting.set(false);
        });
    });

    let pick_services = services.clone();
    let pick_photo = EventHandler::new(move |_| {
        let services = pick_services.clone();
        let suggest = suggest;
        spawn(async move {
            photo_error.set(None);
            let photo = match services.photo_picker.pick_photo().await {
                Ok(Some(photo)) => photo,
                Ok(None) => return,
                Err(failure) => {
                    photo_error.set(Some(failure.into()));
                    return;
                }
            };
            match services
                .image_converter
                .convert_jpeg(photo.bytes, MAX_PHOTO_LONG_SIDE)
                .await
            {
                Ok(image) => {
                    picked.set(Some(image.clone()));
                    remove_photo.set(false);
                    suggest.call(image);
                }
                Err(failure) => photo_error.set(Some(failure.into())),
            }
        });
    });

    let delete_photo = EventHandler::new(move |_| {
        // 飛んでいる推測の応答は捨てる (FR-19)。
        suggestion_generation.set(suggestion_generation() + 1);
        suggesting.set(false);
        suggestion_failed.set(false);
        unmatched_product.set(None);
        picked.set(None);
        if photo_key().is_some() {
            remove_photo.set(true);
        }
    });

    // 推測した内容で商品を登録する (FR-19)。
    let register_services = services.clone();
    let register_product = EventHandler::new(move |_| {
        let input = match crate::records::validate_product_form(
            &draft_name(),
            &draft_producer(),
            &draft_origin(),
            &draft_region(),
            &draft_process(),
            &draft_variety(),
            &draft_tags(),
        ) {
            Ok(input) => input,
            Err(key) => {
                draft_error.set(Some(RecordError::Validation(key)));
                return;
            }
        };
        let api = RecordsApi::new(register_services.api.clone());
        draft_error.set(None);
        spawn(async move {
            match api.create_product(&input).await {
                Ok(created) => {
                    product.set(Some(created));
                    unmatched_product.set(None);
                    register_open.set(false);
                    mark_records_changed(&mut revision);
                }
                Err(failure) => draft_error.set(Some(failure)),
            }
        });
    });

    let save_services = services.clone();
    let save_id = id.clone();
    let save = EventHandler::new(move |_| {
        if busy() {
            return;
        }
        let current_product = product();
        let current_shop = shop();
        let purchased_on_value = purchased_on();
        let roast_value = roast();
        let roast_date_value = roast_date();
        let price_value = price();
        let currency_value = currency();
        let weight_value = weight();
        let input = match validate_purchase_form(PurchaseFormValues {
            product: current_product.as_ref(),
            shop: current_shop.as_ref(),
            purchased_on: &purchased_on_value,
            roast: &roast_value,
            roast_date: &roast_date_value,
            price: &price_value,
            currency: &currency_value,
            weight: &weight_value,
        }) {
            Ok(input) => input,
            Err(found) => {
                errors.set(found);
                save_error.set(None);
                return;
            }
        };
        errors.set(PurchaseFormErrors::default());
        save_error.set(None);
        busy.set(true);
        let api = RecordsApi::new(save_services.api.clone());
        let uploader = save_services.uploader.clone();
        let id = save_id.clone();
        spawn(async move {
            // 登録の後に写真のアップロードで失敗した場合のやり直しでは、同じ購入を更新する。
            let effective_id = id.clone().or(created_id());
            // 写真の削除は購入の更新の前に行う (差し替えでは、この後に新しい写真を紐づける。FR-10)。
            if let Some(current_id) = effective_id.as_ref() {
                if remove_photo() && photo_key().is_some() {
                    match api.delete_photo(current_id).await {
                        Ok(_) => {
                            photo_key.set(None);
                            remove_photo.set(false);
                            mark_records_changed(&mut revision);
                        }
                        Err(failure) => {
                            save_error.set(Some(failure));
                            busy.set(false);
                            return;
                        }
                    }
                }
            }
            let purchase = match save_target(effective_id.as_deref()) {
                SaveTarget::Update(current_id) => api.update_purchase(&current_id, &input).await,
                SaveTarget::Create => api.create_purchase(&input).await,
            };
            let purchase = match purchase {
                Ok(purchase) => purchase,
                Err(failure) => {
                    save_error.set(Some(failure));
                    busy.set(false);
                    return;
                }
            };
            // 写真のアップロードに失敗しても、やり直しで同じ購入を更新できるようにする。
            created_id.set(Some(purchase.id.clone()));
            if let Some(image) = picked() {
                if let Err(failure) = uploader.upload(purchase.id.clone(), image).await {
                    save_error.set(Some(failure));
                    busy.set(false);
                    return;
                }
            }
            mark_records_changed(&mut revision);
            notice.set(Some(t(Key::SavedMessage).to_string()));
            match on_saved {
                Some(handler) => handler.call(()),
                None => navigator.go_back(),
            }
            busy.set(false);
        });
    });

    // 参照先の選択のシートの一覧 (アーカイブ済みは含めない。FR-9)。
    let picker_services = services.clone();
    let product_load = RecordLoader::new({
        let services = picker_services.clone();
        move |cursor, _include_archived| {
            let api = RecordsApi::new(services.api.clone());
            Box::pin(async move { api.products(cursor.as_deref(), false, None).await })
        }
    });
    let shop_load = RecordLoader::new({
        let services = picker_services.clone();
        move |cursor, _include_archived| {
            let api = RecordsApi::new(services.api.clone());
            Box::pin(async move { api.shops(cursor.as_deref(), false).await })
        }
    });
    let product_row = Callback::new(move |choice: Product| {
        let subtitle = choice.producer.clone().filter(|value| !value.is_empty());
        let selected = choice.clone();
        rsx! {
            ListRow {
                title: choice.name.clone(),
                subtitle: subtitle.map(|text| rsx! { span { "{text}" } }),
                on_click: Some(EventHandler::new(move |_| {
                    product.set(Some(selected.clone()));
                    unmatched_product.set(None);
                    errors.set(PurchaseFormErrors::default());
                    picker.set(None);
                })),
            }
        }
    });
    let shop_row = Callback::new(move |choice: Shop| {
        let subtitle = choice.address.clone().filter(|value| !value.is_empty());
        let selected = choice.clone();
        rsx! {
            ListRow {
                title: choice.name.clone(),
                subtitle: subtitle.map(|text| rsx! { span { "{text}" } }),
                on_click: Some(EventHandler::new(move |_| {
                    shop.set(Some(selected.clone()));
                    picker.set(None);
                })),
            }
        }
    });

    let close = EventHandler::new(move |_| match on_close {
        Some(handler) => handler.call(()),
        None => navigator.go_back(),
    });
    let retry = EventHandler::new(move |_| reload.call(()));
    let retry_save = EventHandler::new(move |_| save.call(()));
    let failure = save_error();
    let load_failure = load_error();
    let photo_failure = photo_error();
    let language = current_language();
    let uploaded = !remove_photo() && photo_key().is_some();
    let has_photo = picked().is_some() || uploaded;
    let preview = picked().map(|image| photo_preview_url(&image.bytes));
    let uploaded_url = format!(
        "{}/purchases/{}/photo",
        services.api.base_path(),
        id.clone().or(created_id()).unwrap_or_default()
    );
    let current_product = product();
    let current_shop = shop();
    let unmatched = unmatched_product();
    let currencies = currency_options(Some(&currency()));
    let register_failure = draft_error();

    rsx! {
        div { class: "screen",
            AppBar {
                title: t(if id.is_some() { Key::PurchaseEditTitle } else { Key::PurchaseNewTitle }).to_string(),
                leading_icon: Some("close".to_string()),
                leading_label: Some(t(Key::CancelButton).to_string()),
                on_leading: move |_| close.call(()),
                actions: rsx! {
                    Button {
                        label: t(Key::SaveButton).to_string(),
                        variant: ButtonVariant::Text,
                        disabled: busy(),
                        onclick: move |_| save.call(()),
                    }
                },
            }
            div { class: "body",
                if loading() {
                    div { class: "empty", "{t(Key::Loading)}" }
                } else if let Some(failure) = load_failure {
                    div { class: "form",
                        {retryable_banner(&failure, retry)}
                    }
                } else {
                    div { class: "form",
                        if errors().has_errors() {
                            Banner { message: t(Key::ErrorValidation).to_string() }
                        }
                        div {
                            class: "picker",
                            role: "button",
                            tabindex: "0",
                            onclick: move |_| picker.set(Some(PickerKind::Product)),
                            div { class: "main",
                                div { class: "k",
                                    "{t(Key::ProductLabel)}"
                                    span { class: "req", "{t(Key::RequiredLabel)}" }
                                }
                                div { class: "n",
                                    if let Some(product) = current_product.clone() {
                                        "{product.name}"
                                    } else {
                                        span { class: "faint", "{t(Key::SelectProductTitle)}" }
                                    }
                                }
                            }
                            Icon { name: "chevron_right".to_string(), muted: true }
                        }
                        if let Some(key) = errors().product {
                            div { class: "field-error", "{t(key)}" }
                        }
                        if let Some(suggested) = unmatched.clone() {
                            Button {
                                label: t(Key::SuggestionRegisterProductButton).to_string(),
                                variant: ButtonVariant::Secondary,
                                size: crate::ui::ButtonSize::Sm,
                                disabled: busy() || suggesting(),
                                onclick: move |_| {
                                    draft_name.set(suggested.name.clone().unwrap_or_default());
                                    draft_producer.set(suggested.producer.clone().unwrap_or_default());
                                    draft_origin.set(suggested.origin.clone().unwrap_or_default());
                                    draft_region.set(suggested.region.clone().unwrap_or_default());
                                    draft_process.set(suggested.process.clone().unwrap_or_default());
                                    draft_variety.set(suggested.variety.clone().unwrap_or_default());
                                    // 推測した Flavor Notes も引き継ぐ (Flutter と同じ。
                                    // 0041 のレビューの指摘)。
                                    draft_tags.set(suggested.flavor_notes.clone());
                                    draft_error.set(None);
                                    register_open.set(true);
                                },
                            }
                        }
                        div {
                            class: "picker",
                            role: "button",
                            tabindex: "0",
                            onclick: move |_| picker.set(Some(PickerKind::Shop)),
                            div { class: "main",
                                div { class: "k", "{t(Key::ShopLabel)}" }
                                div { class: "n",
                                    if let Some(shop) = current_shop.clone() {
                                        "{shop.name}"
                                    } else {
                                        span { class: "faint", "{t(Key::ShopNoneLabel)}" }
                                    }
                                }
                            }
                            Icon { name: "chevron_right".to_string(), muted: true }
                        }
                        Field {
                            label: t(Key::PurchasedOnLabel).to_string(),
                            required: true,
                            error: errors().purchased_on.map(|key| t(key).to_string()),
                            disabled: busy(),
                            TextField {
                                value: purchased_on(),
                                mono: true,
                                placeholder: Some(t(Key::DayFormatHint).to_string()),
                                disabled: busy(),
                                oninput: move |event: FormEvent| purchased_on.set(event.value()),
                            }
                        }
                        super::SuggestionField {
                            api: RecordsApi::new(services.api.clone()),
                            target: crate::records::SuggestionTarget::Roast,
                            label: t(Key::Roast).to_string(),
                            value: roast,
                            disabled: busy(),
                        }
                        Field {
                            label: t(Key::RoastDate).to_string(),
                            error: errors().roast_date.map(|key| t(key).to_string()),
                            disabled: busy(),
                            TextField {
                                value: roast_date(),
                                mono: true,
                                placeholder: Some(t(Key::DayFormatHint).to_string()),
                                disabled: busy(),
                                oninput: move |event: FormEvent| roast_date.set(event.value()),
                            }
                        }
                        div { class: "grid2",
                            Field {
                                label: t(Key::PriceLabel).to_string(),
                                error: errors().price.map(|key| t(key).to_string()),
                                disabled: busy(),
                                TextField {
                                    value: price(),
                                    mono: true,
                                    disabled: busy(),
                                    oninput: move |event: FormEvent| price.set(event.value()),
                                }
                            }
                            Field { label: t(Key::CurrencyLabel).to_string(), disabled: busy(),
                                div { class: "box",
                                    select {
                                        class: "in",
                                        disabled: busy(),
                                        onchange: move |event: FormEvent| currency.set(event.value()),
                                        for code in currencies {
                                            option {
                                                value: "{code}",
                                                selected: code == currency(),
                                                "{currency_option_label(language, &code)}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Field {
                            label: t(Key::WeightLabel).to_string(),
                            error: errors().weight.map(|key| t(key).to_string()),
                            disabled: busy(),
                            TextField {
                                value: weight(),
                                mono: true,
                                unit: Some(t(Key::GramUnit).to_string()),
                                disabled: busy(),
                                oninput: move |event: FormEvent| weight.set(event.value()),
                            }
                        }
                        Field { label: t(Key::PhotoLabel).to_string(), disabled: busy(),
                            div { class: "photo-row",
                                div { class: "photo",
                                    style: "width: 88px; height: 88px;",
                                    if let Some(preview) = preview.clone() {
                                        img { src: preview, alt: t(Key::PhotoLabel).to_string() }
                                    } else if uploaded {
                                        img { src: uploaded_url.clone(), alt: t(Key::PhotoLabel).to_string() }
                                    } else {
                                        Icon { name: "photo_camera".to_string(), muted: true }
                                    }
                                }
                                div { class: "photo-acts",
                                    Button {
                                        label: t(if has_photo { Key::PhotoReplaceButton } else { Key::PhotoSelectButton }).to_string(),
                                        variant: ButtonVariant::Secondary,
                                        size: crate::ui::ButtonSize::Sm,
                                        disabled: busy(),
                                        onclick: move |_| pick_photo.call(()),
                                    }
                                    if has_photo {
                                        Button {
                                            label: t(Key::PhotoDeleteButton).to_string(),
                                            variant: ButtonVariant::Text,
                                            disabled: busy(),
                                            onclick: move |_| delete_photo.call(()),
                                        }
                                    }
                                    p { class: "t-caption muted", "{t(Key::PhotoConvertNote)}" }
                                }
                            }
                        }
                        if suggesting() {
                            div { class: "t-caption muted", "{t(Key::SuggestionLoadingLabel)}" }
                        }
                        if suggestion_failed() {
                            Banner { message: t(Key::SuggestionFailedMessage).to_string() }
                        }
                        if let Some(failure) = photo_failure {
                            {retryable_banner(&failure, EventHandler::new(move |_| pick_photo.call(())))}
                        }
                        if let Some(failure) = failure {
                            {retryable_banner(&failure, retry_save)}
                        }
                    }
                }
            }
        }
        if let Some(kind) = picker() {
            if kind == PickerKind::Product {
                RecordPickerSheet::<Product> {
                    title: t(Key::SelectProductTitle).to_string(),
                    load: product_load,
                    row: product_row,
                    on_close: EventHandler::new(move |_| picker.set(None)),
                }
            } else {
                RecordPickerSheet::<Shop> {
                    title: t(Key::SelectShopTitle).to_string(),
                    load: shop_load,
                    row: shop_row,
                    clear_label: Some(t(Key::ShopNoneLabel).to_string()),
                    on_clear: EventHandler::new(move |_| {
                        shop.set(None);
                        picker.set(None);
                    }),
                    on_close: EventHandler::new(move |_| picker.set(None)),
                }
            }
        }
        if register_open() {
            div { class: "sheet-scrim",
                div { class: "sheet",
                    div { class: "ttl", "{t(Key::SuggestionRegisterProductButton)}" }
                    div { class: "form",
                        if let Some(failure) = register_failure {
                            Banner { message: t(record_error_key(&failure)).to_string() }
                        }
                        Field {
                            label: t(Key::ProductNameLabel).to_string(),
                            required: true,
                            disabled: busy(),
                            TextField {
                                value: draft_name(),
                                disabled: busy(),
                                oninput: move |event: FormEvent| draft_name.set(event.value()),
                            }
                        }
                        Field { label: t(Key::Producer).to_string(), disabled: busy(),
                            TextField {
                                value: draft_producer(),
                                disabled: busy(),
                                oninput: move |event: FormEvent| draft_producer.set(event.value()),
                            }
                        }
                        div { class: "grid2",
                            Field { label: t(Key::Origin).to_string(), disabled: busy(),
                                TextField {
                                    value: draft_origin(),
                                    disabled: busy(),
                                    oninput: move |event: FormEvent| draft_origin.set(event.value()),
                                }
                            }
                            Field { label: t(Key::Region).to_string(), disabled: busy(),
                                TextField {
                                    value: draft_region(),
                                    disabled: busy(),
                                    oninput: move |event: FormEvent| draft_region.set(event.value()),
                                }
                            }
                        }
                        div { class: "grid2",
                            Field { label: t(Key::Process).to_string(), disabled: busy(),
                                TextField {
                                    value: draft_process(),
                                    disabled: busy(),
                                    oninput: move |event: FormEvent| draft_process.set(event.value()),
                                }
                            }
                            Field { label: t(Key::Variety).to_string(), disabled: busy(),
                                TextField {
                                    value: draft_variety(),
                                    disabled: busy(),
                                    oninput: move |event: FormEvent| draft_variety.set(event.value()),
                                }
                            }
                        }
                    }
                    div { class: "acts",
                        Button {
                            label: t(Key::CancelButton).to_string(),
                            variant: ButtonVariant::Text,
                            onclick: move |_| register_open.set(false),
                        }
                        Button {
                            label: t(Key::SaveButton).to_string(),
                            disabled: busy(),
                            onclick: move |_| register_product.call(()),
                        }
                    }
                }
            }
        }
        if let Some(message) = notice() {
            {clear_notice_after(notice)}
            div { class: "notice",
                crate::ui::Snackbar { message }
            }
        }
    }
}

/// 参照先の選択のシートの種類。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PickerKind {
    /// 商品の選択 (FR-9)。
    Product,
    /// 店の選択 (FR-9)。
    Shop,
}
