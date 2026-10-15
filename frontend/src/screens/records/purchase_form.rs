//! 購入の登録と編集の画面 (FR-9、FR-10、FR-19)。
//!
//! 商品名は必須で、店は省略できる。商品の項目 (生産者、生産国、地域、精製方法、品種、
//! Flavor Notes) も購入のフォームが持つ (ADR-0022)。購入日は端末のタイムゾーンでの当日を
//! 既定値にする (FR-9)。写真は選択した時点では変換と推測だけを行い、購入を保存した後に
//! アップロードする (FR-10、ADR-0003)。推測できた項目は入力済みの値も置き換える (FR-19)。

use dioxus::prelude::*;
use dioxus_router::navigator;

use crate::i18n::{current_language, t, Key};
use crate::records::values::format_day;
use crate::records::{
    apply_purchase_suggestion, currency_option_label, currency_options, save_target,
    validate_purchase_form, ConvertedImage, PurchaseFormErrors, PurchaseFormFields,
    PurchaseFormValues, RecordError, RecordServices, RecordsApi, SaveTarget, Shop,
    SuggestionTarget, DEFAULT_CURRENCY, MAX_PHOTO_BYTES, MAX_PHOTO_LONG_SIDE,
};
use crate::screens::ScreenAppBar;
use crate::ui::{
    Banner, Button, ButtonSize, ButtonVariant, Chip, ChipVariant, Field, Icon, ListRow, TextField,
    TextFieldKind,
};

use super::{
    clear_notice_after, mark_records_changed, photo_preview_url, retryable_banner, DiscardConfirm,
    RecordLoader, RecordPickerSheet, SuggestionField,
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
    let mut name = use_signal(String::new);
    let mut producer = use_signal(String::new);
    let mut origin = use_signal(String::new);
    let mut region = use_signal(String::new);
    let mut process = use_signal(String::new);
    let mut variety = use_signal(String::new);
    let mut tags = use_signal(Vec::<String>::new);
    let mut tag_input = use_signal(String::new);
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
    let mut picker_open = use_signal(|| false);
    let mut suggesting = use_signal(|| false);
    let mut suggestion_failed = use_signal(|| false);
    let mut suggestion_generation = use_signal(|| 0_u64);
    let mut created_id = use_signal(|| None::<String>);
    let mut dirty = use_signal(|| false);
    let mut discard_open = use_signal(|| false);

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
                    name.set(purchase.name.clone());
                    producer.set(purchase.producer.clone().unwrap_or_default());
                    origin.set(purchase.origin.clone().unwrap_or_default());
                    region.set(purchase.region.clone().unwrap_or_default());
                    process.set(purchase.process.clone().unwrap_or_default());
                    variety.set(purchase.variety.clone().unwrap_or_default());
                    tags.set(purchase.flavor_notes.clone());
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
            // 読み込みで入った値は変更に数えない (0054)。
            dirty.set(false);
        });
    });
    use_effect(move || reload.call(()));

    // 写真を選び、JPEG に変換して長辺を縮める (FR-10)。変換の直後に推測を呼ぶ (FR-19)。
    let photo_services = services.clone();
    let suggest = EventHandler::new(move |image: ConvertedImage| {
        // 写真を選び直した時点で世代を進め、飛んでいる古い応答を無効にする (FR-19)。
        let generation = suggestion_generation() + 1;
        suggestion_generation.set(generation);
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
            // 推測できた項目は入力済みの値も置き換える (FR-19、ADR-0022)。
            let mut name_value = name();
            let mut producer_value = producer();
            let mut origin_value = origin();
            let mut region_value = region();
            let mut process_value = process();
            let mut variety_value = variety();
            let mut tags_value = tags();
            let mut roast_value = roast();
            let mut roast_date_value = roast_date();
            let mut price_value = price();
            let mut weight_value = weight();
            apply_purchase_suggestion(
                PurchaseFormFields {
                    name: &mut name_value,
                    producer: &mut producer_value,
                    origin: &mut origin_value,
                    region: &mut region_value,
                    process: &mut process_value,
                    variety: &mut variety_value,
                    flavor_notes: &mut tags_value,
                    roast: &mut roast_value,
                    roast_date: &mut roast_date_value,
                    price: &mut price_value,
                    weight: &mut weight_value,
                },
                &suggestion,
            );
            name.set(name_value);
            producer.set(producer_value);
            origin.set(origin_value);
            region.set(region_value);
            process.set(process_value);
            variety.set(variety_value);
            tags.set(tags_value);
            roast.set(roast_value);
            roast_date.set(roast_date_value);
            price.set(price_value);
            weight.set(weight_value);
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
                    dirty.set(true);
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
        picked.set(None);
        if photo_key().is_some() {
            remove_photo.set(true);
        }
        dirty.set(true);
    });

    // Flavor Notes のタグを足す (FR-8)。同じ名前は 1 つにまとめる。
    let add_tag = EventHandler::new(move |_| {
        let value = tag_input().trim().to_string();
        if value.is_empty() {
            return;
        }
        let mut current = tags();
        if !current.contains(&value) {
            current.push(value);
            tags.set(current);
            dirty.set(true);
        }
        tag_input.set(String::new());
    });

    let save_services = services.clone();
    let save_id = id.clone();
    let save = EventHandler::new(move |_| {
        if busy() {
            return;
        }
        let name_value = name();
        let producer_value = producer();
        let origin_value = origin();
        let region_value = region();
        let process_value = process();
        let variety_value = variety();
        let tags_value = tags();
        let current_shop = shop();
        let purchased_on_value = purchased_on();
        let roast_value = roast();
        let roast_date_value = roast_date();
        let price_value = price();
        let currency_value = currency();
        let weight_value = weight();
        let input = match validate_purchase_form(PurchaseFormValues {
            name: &name_value,
            producer: &producer_value,
            origin: &origin_value,
            region: &region_value,
            process: &process_value,
            variety: &variety_value,
            flavor_notes: &tags_value,
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

    // 店の選択のシートの一覧 (FR-9)。
    let picker_services = services.clone();
    let shop_load = RecordLoader::new({
        let services = picker_services.clone();
        move |cursor, options| {
            let api = RecordsApi::new(services.api.clone());
            Box::pin(async move { api.shops(&options, cursor.as_deref()).await })
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
                    picker_open.set(false);
                    dirty.set(true);
                })),
            }
        }
    });

    // 変更があるときは、閉じる前に破棄の確認を出す (0054)。
    let close_now = EventHandler::new(move |_| match on_close {
        Some(handler) => handler.call(()),
        None => navigator.go_back(),
    });
    let request_close = EventHandler::new(move |_| {
        if dirty() {
            discard_open.set(true);
        } else {
            close_now.call(());
        }
    });
    let confirm_discard = EventHandler::new(move |_| close_now.call(()));
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
    let current_shop = shop();
    let currencies = currency_options(Some(&currency()));

    rsx! {
        div { class: "screen",
            ScreenAppBar {
                title: t(if id.is_some() { Key::PurchaseEditTitle } else { Key::PurchaseNewTitle }).to_string(),
                menu: false,
                leading_icon: Some("close".to_string()),
                leading_label: Some(t(Key::CancelButton).to_string()),
                on_leading: move |_| request_close.call(()),
                actions: rsx! {
                    Button {
                        label: t(Key::SaveButton).to_string(),
                        variant: ButtonVariant::Primary,
                        size: ButtonSize::Sm,
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
                        Field {
                            label: t(Key::ProductNameLabel).to_string(),
                            required: true,
                            error: errors().name.map(|key| t(key).to_string()),
                            disabled: busy(),
                            TextField {
                                value: name(),
                                disabled: busy(),
                                oninput: move |event: FormEvent| {
                                    name.set(event.value());
                                    dirty.set(true);
                                },
                            }
                        }
                        SuggestionField {
                            api: RecordsApi::new(services.api.clone()),
                            target: SuggestionTarget::Producer,
                            label: t(Key::Producer).to_string(),
                            value: producer,
                            disabled: busy(),
                            on_change: move |_| dirty.set(true),
                        }
                        div { class: "grid2",
                            SuggestionField {
                                api: RecordsApi::new(services.api.clone()),
                                target: SuggestionTarget::Origin,
                                label: t(Key::Origin).to_string(),
                                value: origin,
                                disabled: busy(),
                                on_change: move |_| dirty.set(true),
                            }
                            SuggestionField {
                                api: RecordsApi::new(services.api.clone()),
                                target: SuggestionTarget::Region,
                                label: t(Key::Region).to_string(),
                                value: region,
                                disabled: busy(),
                                on_change: move |_| dirty.set(true),
                            }
                        }
                        div { class: "grid2",
                            SuggestionField {
                                api: RecordsApi::new(services.api.clone()),
                                target: SuggestionTarget::Process,
                                label: t(Key::Process).to_string(),
                                value: process,
                                disabled: busy(),
                                on_change: move |_| dirty.set(true),
                            }
                            SuggestionField {
                                api: RecordsApi::new(services.api.clone()),
                                target: SuggestionTarget::Variety,
                                label: t(Key::Variety).to_string(),
                                value: variety,
                                disabled: busy(),
                                on_change: move |_| dirty.set(true),
                            }
                        }
                        Field { label: t(Key::FlavorNotes).to_string(), disabled: busy(),
                            div { class: "chips",
                                for note in tags() {
                                    Chip {
                                        label: note.clone(),
                                        variant: ChipVariant::Tag,
                                        on_remove: Some(EventHandler::new(move |_| {
                                            let removed = note.clone();
                                            tags.set(
                                                tags().into_iter().filter(|tag| tag != &removed).collect(),
                                            );
                                            dirty.set(true);
                                        })),
                                    }
                                }
                            }
                            div { class: "tag-add",
                                TextField {
                                    value: tag_input(),
                                    placeholder: Some(t(Key::TagInputHint).to_string()),
                                    disabled: busy(),
                                    oninput: move |event: FormEvent| {
                                        tag_input.set(event.value());
                                        dirty.set(true);
                                    },
                                }
                                Button {
                                    label: t(Key::AddButton).to_string(),
                                    variant: ButtonVariant::Secondary,
                                    size: ButtonSize::Sm,
                                    disabled: busy(),
                                    onclick: move |_| add_tag.call(()),
                                }
                            }
                        }
                        div {
                            class: "picker",
                            role: "button",
                            tabindex: "0",
                            onclick: move |_| picker_open.set(true),
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
                            help: Some(t(Key::DayFormatHint).to_string()),
                            disabled: busy(),
                            TextField {
                                value: purchased_on(),
                                mono: true,
                                kind: TextFieldKind::Date,
                                disabled: busy(),
                                oninput: move |event: FormEvent| {
                                    purchased_on.set(event.value());
                                    dirty.set(true);
                                },
                            }
                        }
                        SuggestionField {
                            api: RecordsApi::new(services.api.clone()),
                            target: SuggestionTarget::Roast,
                            label: t(Key::Roast).to_string(),
                            value: roast,
                            disabled: busy(),
                            on_change: move |_| dirty.set(true),
                        }
                        Field {
                            label: t(Key::RoastDate).to_string(),
                            error: errors().roast_date.map(|key| t(key).to_string()),
                            help: Some(t(Key::DayFormatHint).to_string()),
                            disabled: busy(),
                            TextField {
                                value: roast_date(),
                                mono: true,
                                kind: TextFieldKind::Date,
                                disabled: busy(),
                                oninput: move |event: FormEvent| {
                                    roast_date.set(event.value());
                                    dirty.set(true);
                                },
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
                                    oninput: move |event: FormEvent| {
                                        price.set(event.value());
                                        dirty.set(true);
                                    },
                                }
                            }
                            Field { label: t(Key::CurrencyLabel).to_string(), disabled: busy(),
                                div { class: "box",
                                    select {
                                        class: "in",
                                        disabled: busy(),
                                        onchange: move |event: FormEvent| {
                                            currency.set(event.value());
                                            dirty.set(true);
                                        },
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
                                oninput: move |event: FormEvent| {
                                    weight.set(event.value());
                                    dirty.set(true);
                                },
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
        if picker_open() {
            RecordPickerSheet::<Shop> {
                title: t(Key::SelectShopTitle).to_string(),
                load: shop_load,
                row: shop_row,
                clear_label: Some(t(Key::ShopNoneLabel).to_string()),
                on_clear: EventHandler::new(move |_| {
                    shop.set(None);
                    picker_open.set(false);
                    dirty.set(true);
                }),
                on_close: EventHandler::new(move |_| picker_open.set(false)),
            }
        }
        if discard_open() {
            DiscardConfirm { open: discard_open, on_discard: confirm_discard }
        }
        if let Some(message) = notice() {
            {clear_notice_after(notice)}
            div { class: "notice",
                crate::ui::Snackbar { message }
            }
        }
    }
}
