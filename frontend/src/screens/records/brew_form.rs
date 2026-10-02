//! 抽出の登録と編集の画面 (FR-11)。
//!
//! 購入は必須で、ボトムシートから選ぶ。抽出日時は端末のローカル時刻で入力し、送信の直前に
//! UTC の ISO 8601 へ変換する (FR-11)。抽出方法と挽き目は、入力中に過去の入力値の候補を出す
//! (FR-13)。保存は AppBar の右端の文字ボタン。1 画面に収める (docs/design/components/BrewForm)。

use dioxus::prelude::*;
use dioxus_router::navigator;

use crate::i18n::{current_language, t, Key};
use crate::records::display::purchase_row_subtitle;
use crate::records::values::{format_day, format_number, format_time, parse_utc_to_local};
use crate::records::{
    save_target, validate_brew_form, BrewFormErrors, BrewFormValues, Purchase, RecordError,
    RecordServices, RecordsApi, SaveTarget, SuggestionTarget,
};
use crate::ui::{
    AppBar, Banner, Button, ButtonVariant, Field, Icon, ListRow, RatingInput, RowValue, TextField,
};

use super::{
    clear_notice_after, mark_records_changed, retryable_banner, RecordLoader, RecordPickerSheet,
    SuggestionField,
};

/// 抽出の登録 (FR-11)。
#[component]
pub fn BrewFormScreen() -> Element {
    rsx! {
        BrewForm {}
    }
}

/// 抽出の編集 (FR-11)。
#[component]
pub fn BrewEditScreen(id: String) -> Element {
    rsx! {
        BrewForm { id: Some(id) }
    }
}

/// 抽出の登録と編集のフォーム。2 段組の右の面にも出せる。
#[component]
pub fn BrewForm(
    /// 編集する抽出の ID。新規の登録のときは None。
    #[props(default)]
    id: Option<String>,

    /// 2 段組の右の面に出すか。保存と閉じるの動きを親が受け取る。
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

    // 既定値は端末のタイムゾーンでの現在の日時とする (FR-11)。
    let now = services.clock.now();
    let mut purchase = use_signal(|| None::<Purchase>);
    let mut date = use_signal(|| format_day(now.date()));
    let mut time = use_signal(|| format_time(now.hour, now.minute));
    let mut dose = use_signal(String::new);
    let mut water = use_signal(String::new);
    let mut water_temp = use_signal(String::new);
    let mut brew_time = use_signal(String::new);
    let mut method = use_signal(String::new);
    let mut grind_setting = use_signal(String::new);
    let mut rating = use_signal(|| None::<u8>);
    let mut notes = use_signal(String::new);
    let mut errors = use_signal(BrewFormErrors::default);
    let mut save_error = use_signal(|| None::<RecordError>);
    let mut load_error = use_signal(|| None::<RecordError>);
    let mut loading = use_signal(|| id.is_some());
    let mut busy = use_signal(|| false);
    let mut picker_open = use_signal(|| false);

    // 編集のために現在の値を読み込む。再試行でも同じ処理を呼ぶ。
    let reload_services = services.clone();
    let reload_id = id.clone();
    let reload = EventHandler::new(move |_| {
        let Some(id) = reload_id.clone() else {
            return;
        };
        let api = RecordsApi::new(reload_services.api.clone());
        let offset = reload_services.clock.utc_offset_minutes();
        spawn(async move {
            loading.set(true);
            load_error.set(None);
            match api.brew(&id).await {
                Ok(brew) => {
                    purchase.set(Some(brew.purchase.clone()));
                    if let Some(local) = parse_utc_to_local(&brew.brewed_at, offset) {
                        date.set(format_day(local.date()));
                        time.set(format_time(local.hour, local.minute));
                    }
                    dose.set(brew.dose_grams.map(format_number).unwrap_or_default());
                    water.set(brew.water_grams.map(format_number).unwrap_or_default());
                    water_temp.set(brew.water_temp_c.map(format_number).unwrap_or_default());
                    brew_time.set(
                        brew.brew_time_seconds
                            .map(|value| value.to_string())
                            .unwrap_or_default(),
                    );
                    method.set(brew.method.clone().unwrap_or_default());
                    grind_setting.set(brew.grind_setting.clone().unwrap_or_default());
                    rating.set(brew.rating);
                    notes.set(brew.notes.clone().unwrap_or_default());
                }
                Err(failure) => load_error.set(Some(failure)),
            }
            loading.set(false);
        });
    });
    use_effect(move || reload.call(()));

    let save_services = services.clone();
    let save_id = id.clone();
    let save = EventHandler::new(move |_| {
        if busy() {
            return;
        }
        let current_purchase = purchase();
        let date_value = date();
        let time_value = time();
        let dose_value = dose();
        let water_value = water();
        let water_temp_value = water_temp();
        let brew_time_value = brew_time();
        let method_value = method();
        let grind_setting_value = grind_setting();
        let notes_value = notes();
        let values = BrewFormValues {
            purchase: current_purchase.as_ref(),
            date: &date_value,
            time: &time_value,
            dose: &dose_value,
            water: &water_value,
            water_temp: &water_temp_value,
            brew_time: &brew_time_value,
            method: &method_value,
            grind_setting: &grind_setting_value,
            rating: rating(),
            notes: &notes_value,
            utc_offset_minutes: save_services.clock.utc_offset_minutes(),
        };
        let input = match validate_brew_form(values) {
            Ok(input) => input,
            Err(found) => {
                errors.set(found);
                save_error.set(None);
                return;
            }
        };
        errors.set(BrewFormErrors::default());
        save_error.set(None);
        busy.set(true);
        let api = RecordsApi::new(save_services.api.clone());
        let id = save_id.clone();
        spawn(async move {
            let result = match save_target(id.as_deref()) {
                SaveTarget::Update(id) => api.update_brew(&id, &input).await,
                SaveTarget::Create => api.create_brew(&input).await,
            };
            match result {
                Ok(_) => {
                    notice.set(Some(t(Key::SavedMessage).to_string()));
                    mark_records_changed(&mut revision);
                    match on_saved {
                        Some(handler) => handler.call(()),
                        None => navigator.go_back(),
                    }
                }
                Err(failure) => save_error.set(Some(failure)),
            }
            busy.set(false);
        });
    });

    // 購入の選択のシートの一覧 (アーカイブ済みは含めない。FR-11)。
    let picker_services = services.clone();
    let picker_load = RecordLoader::new(move |cursor, _include_archived| {
        let api = RecordsApi::new(picker_services.api.clone());
        Box::pin(async move { api.purchases(cursor.as_deref(), false).await })
    });
    let picker_row = Callback::new(move |choice: Purchase| {
        let caption = purchase_row_subtitle(&choice, current_language());
        let selected = choice.clone();
        rsx! {
            ListRow {
                title: choice.product.name.clone(),
                subtitle: Some(rsx! { span { RowValue { text: caption } } }),
                on_click: Some(EventHandler::new(move |_| {
                    purchase.set(Some(selected.clone()));
                    picker_open.set(false);
                    errors.set(BrewFormErrors::default());
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
    let current_purchase = purchase();
    let failure = save_error();
    let load_failure = load_error();

    rsx! {
        div { class: "screen",
            AppBar {
                title: t(if id.is_some() { Key::BrewEditTitle } else { Key::BrewNewTitle }).to_string(),
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
                            onclick: move |_| picker_open.set(true),
                            div { class: "main",
                                div { class: "k",
                                    "{t(Key::PurchaseLabel)}"
                                    span { class: "req", "{t(Key::RequiredLabel)}" }
                                }
                                div { class: "n",
                                    if let Some(purchase) = current_purchase.clone() {
                                        "{purchase.product.name}"
                                    } else {
                                        span { class: "faint", "{t(Key::PurchasePickPlaceholder)}" }
                                    }
                                }
                                if let Some(purchase) = current_purchase.clone() {
                                    div { class: "s", {purchase_row_subtitle(&purchase, current_language())} }
                                }
                            }
                            Icon { name: "chevron_right".to_string(), muted: true }
                        }
                        if let Some(key) = errors().purchase {
                            div { class: "field-error", "{t(key)}" }
                        }
                        div { class: "grid2",
                            Field {
                                label: t(Key::BrewedAtLabel).to_string(),
                                required: true,
                                error: errors().date.map(|key| t(key).to_string()),
                                disabled: busy(),
                                TextField {
                                    value: date(),
                                    mono: true,
                                    placeholder: Some(t(Key::DayFormatHint).to_string()),
                                    disabled: busy(),
                                    oninput: move |event: FormEvent| date.set(event.value()),
                                }
                            }
                            Field {
                                label: String::new(),
                                error: errors().time.map(|key| t(key).to_string()),
                                disabled: busy(),
                                TextField {
                                    value: time(),
                                    mono: true,
                                    placeholder: Some(t(Key::TimeFormatHint).to_string()),
                                    disabled: busy(),
                                    oninput: move |event: FormEvent| time.set(event.value()),
                                }
                            }
                        }
                        div { class: "grid2",
                            Field {
                                label: t(Key::DoseLabel).to_string(),
                                error: errors().dose.map(|key| t(key).to_string()),
                                disabled: busy(),
                                TextField {
                                    value: dose(),
                                    mono: true,
                                    unit: Some(t(Key::GramUnit).to_string()),
                                    disabled: busy(),
                                    oninput: move |event: FormEvent| dose.set(event.value()),
                                }
                            }
                            Field {
                                label: t(Key::WaterLabel).to_string(),
                                error: errors().water.map(|key| t(key).to_string()),
                                disabled: busy(),
                                TextField {
                                    value: water(),
                                    mono: true,
                                    unit: Some(t(Key::GramUnit).to_string()),
                                    disabled: busy(),
                                    oninput: move |event: FormEvent| water.set(event.value()),
                                }
                            }
                        }
                        div { class: "grid2",
                            Field {
                                label: t(Key::WaterTempLabel).to_string(),
                                error: errors().water_temp.map(|key| t(key).to_string()),
                                disabled: busy(),
                                TextField {
                                    value: water_temp(),
                                    mono: true,
                                    unit: Some(t(Key::CelsiusUnit).to_string()),
                                    disabled: busy(),
                                    oninput: move |event: FormEvent| water_temp.set(event.value()),
                                }
                            }
                            Field {
                                label: t(Key::BrewTimeLabel).to_string(),
                                error: errors().brew_time.map(|key| t(key).to_string()),
                                disabled: busy(),
                                TextField {
                                    value: brew_time(),
                                    mono: true,
                                    unit: Some(t(Key::SecondUnit).to_string()),
                                    disabled: busy(),
                                    oninput: move |event: FormEvent| brew_time.set(event.value()),
                                }
                            }
                        }
                        SuggestionField {
                            api: RecordsApi::new(services.api.clone()),
                            target: SuggestionTarget::Method,
                            label: t(Key::MethodLabel).to_string(),
                            value: method,
                            disabled: busy(),
                        }
                        SuggestionField {
                            api: RecordsApi::new(services.api.clone()),
                            target: SuggestionTarget::GrindSetting,
                            label: t(Key::GrindSettingLabel).to_string(),
                            value: grind_setting,
                            hint: Some(t(Key::GrindSettingHint).to_string()),
                            disabled: busy(),
                        }
                        Field { label: t(Key::RatingLabel).to_string(), disabled: busy(),
                            RatingInput {
                                value: rating(),
                                enabled: !busy(),
                                on_change: move |value| rating.set(Some(value)),
                            }
                        }
                        Field { label: t(Key::NotesLabel).to_string(), disabled: busy(),
                            TextField {
                                value: notes(),
                                area: true,
                                disabled: busy(),
                                oninput: move |event: FormEvent| notes.set(event.value()),
                            }
                        }
                        if let Some(failure) = failure {
                            {retryable_banner(&failure, retry_save)}
                        }
                    }
                }
            }
        }
        if picker_open() {
            RecordPickerSheet::<Purchase> {
                title: t(Key::SelectPurchaseTitle).to_string(),
                load: picker_load,
                row: picker_row,
                on_close: EventHandler::new(move |_| picker_open.set(false)),
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
