//! 購入の詳細の画面 (FR-9、FR-10、UC-6)。
//!
//! 店をたどれるようにし、写真の差し替えと削除ができる (FR-10)。幅 840 px 以上の
//! 2 段組では `embedded` を true にし、右の面に出す。帯の右端の操作はお気に入り、
//! この購入をもとに登録 (issue 0058)、編集、削除の順。

use dioxus::prelude::*;
use dioxus_router::navigator;

use crate::i18n::{current_language, t, t_args, Key};
use crate::records::display::{
    purchase_reference_tiles, purchase_row_subtitle, purchase_tile_name,
};
use crate::records::stats::RatingHistoryEntry;
use crate::records::{
    record_error_key, record_error_not_found, record_error_retry, DeleteImpact, RecordError,
    RecordServices, RecordsApi, StatsApi, TasteProfile, MAX_PHOTO_LONG_SIDE,
};
use crate::router::Route;
use crate::screens::stats::RatingHistoryChart;
use crate::screens::ScreenAppBar;
use crate::ui::{
    Banner, Button, ButtonSize, ButtonVariant, IconButton, Ledger, LedgerRow, ReferenceChain,
    ReferenceTile, TagChip,
};

use super::{clear_notice_after, mark_records_changed, retryable_banner, DeleteConfirm};

/// 購入の詳細 (FR-9)。
#[component]
pub fn PurchaseDetailScreen(id: String) -> Element {
    rsx! {
        // 1 件の一覧として鍵を付け、ID が変わったら詳細を作り直す。
        for current in [id] {
            PurchaseDetail { key: "{current}", id: current }
        }
    }
}

/// 購入の詳細の中身。2 段組の右の面にも出せる。
///
/// `id` が変わるときは、呼び出し側が keyed な 1 件の一覧として作り直すこと。
/// 読み込みはマウント時と記録の変更時に行い、`id` の変更では作り直しに頼る。
#[component]
pub fn PurchaseDetail(
    /// 表示する購入の ID。
    id: String,

    /// 2 段組の右の面に出すか。
    #[props(default = false)]
    embedded: bool,

    /// 編集を開く動き。無いときは編集の経路を上に積む。
    #[props(default)]
    on_edit: Option<EventHandler<()>>,

    /// この購入をもとに新規のフォームを開く動き (issue 0058)。無いときは登録の経路
    /// (`/purchases/new?from=<id>`) を上に積む。
    #[props(default)]
    on_duplicate: Option<EventHandler<()>>,

    /// 削除できたときの動き。無いときは前の画面へ戻る。
    #[props(default)]
    on_deleted: Option<EventHandler<()>>,
) -> Element {
    let services = use_context::<RecordServices>();
    let mut revision = use_context::<Signal<u64>>();
    let notice = use_context::<Signal<Option<String>>>();
    let navigator = navigator();
    let mut purchase = use_signal(|| None::<crate::records::Purchase>);
    let mut ratings = use_signal(Vec::<RatingHistoryEntry>::new);
    let mut error = use_signal(|| None::<RecordError>);
    let mut ratings_error = use_signal(|| None::<RecordError>);
    // AI による味と特徴の推定 (FR-24)。利用者が押したときだけ推定し、保存はしない。
    let mut taste = use_signal(|| None::<TasteProfile>);
    let mut taste_error = use_signal(|| None::<RecordError>);
    let mut taste_busy = use_signal(|| false);
    let mut taste_generation = use_signal(|| 0_u64);
    let mut photo_error = use_signal(|| None::<RecordError>);
    let mut photo_busy = use_signal(|| false);
    let mut loading = use_signal(|| true);
    // 削除の確認と、削除の影響と失敗の再試行 (0056)。
    let mut delete_open = use_signal(|| false);
    let mut delete_impact = use_signal(|| None::<DeleteImpact>);
    let mut impact_error = use_signal(|| None::<RecordError>);
    let mut delete_error = use_signal(|| None::<RecordError>);
    // 購入を切り替えたときに、前の購入の読み込みを混ぜないための世代。
    let mut reload_generation = use_signal(|| 0_u64);

    let reload_services = services.clone();
    let reload = EventHandler::new(move |current_id: String| {
        let api = RecordsApi::new(reload_services.api.clone());
        let stats = StatsApi::new(reload_services.api.clone());
        let generation = *reload_generation.peek() + 1;
        reload_generation.set(generation);
        spawn(async move {
            loading.set(true);
            error.set(None);
            ratings_error.set(None);
            match api.purchase(&current_id).await {
                Ok(loaded) => {
                    if *reload_generation.peek() != generation {
                        return;
                    }
                    let saved_taste = loaded.taste_profile.clone();
                    purchase.set(Some(loaded));
                    // 保存済みの推定は、まだ画面に出していないときだけ出す。生成し直した
                    // 推定や、お気に入りの切り替えで読み直す前の表示を上書きしない (FR-24)。
                    if taste.peek().is_none() {
                        taste.set(saved_taste);
                    }
                }
                Err(failure) => {
                    if *reload_generation.peek() != generation {
                        return;
                    }
                    error.set(Some(failure));
                }
            }
            // 評価の推移の失敗は、購入の表示を残したままグラフの区画にだけ出す (FR-18)。
            match stats.rating_history(&current_id).await {
                Ok(loaded) => {
                    if *reload_generation.peek() != generation {
                        return;
                    }
                    ratings.set(loaded);
                }
                Err(failure) => {
                    if *reload_generation.peek() != generation {
                        return;
                    }
                    ratings_error.set(Some(failure));
                }
            }
            loading.set(false);
        });
    });
    let retry_reload = {
        let id = id.clone();
        EventHandler::new(move |_| reload.call(id.clone()))
    };
    // マウント時と、記録が変わったときに読み直す。
    let effect_id = id.clone();
    use_effect(move || {
        let _ = revision();
        reload.call(effect_id.clone());
    });

    // 豆の情報から味と特徴を推定する (FR-24)。推定は利用者が押したときだけ行い、
    // 画面の状態にだけ持つ (どの記録も変更しない。ADR-0023)。
    let taste_services = services.clone();
    let taste_id = id.clone();
    let load_taste = EventHandler::new(move |_| {
        // 押し直しや、ボタンの無効化が反映される前の連打で、AI を二重に呼ばない。
        if taste_busy() {
            return;
        }
        let api = RecordsApi::new(taste_services.api.clone());
        let id = taste_id.clone();
        let lang = current_language().code();
        let generation = taste_generation() + 1;
        taste_generation.set(generation);
        taste_busy.set(true);
        taste_error.set(None);
        spawn(async move {
            let result = api.taste_profile(&id, lang).await;
            // 画面を離れた後や、押し直した後の古い応答は捨てる。
            if taste_generation() != generation {
                return;
            }
            match result {
                Ok(profile) => taste.set(Some(profile)),
                Err(failure) => taste_error.set(Some(failure)),
            }
            taste_busy.set(false);
        });
    });

    // 写真を選び直してアップロードする (FR-10)。
    let replace_services = services.clone();
    let replace_id = id.clone();
    let replace_photo = EventHandler::new(move |_| {
        let services = replace_services.clone();
        let id = replace_id.clone();
        spawn(async move {
            photo_busy.set(true);
            photo_error.set(None);
            let result = async {
                let photo = services.photo_picker.pick_photo().await?.ok_or_else(|| {
                    RecordError::Photo(crate::records::PhotoError::new("cancelled"))
                })?;
                let image = services
                    .image_converter
                    .convert_jpeg(photo.bytes, MAX_PHOTO_LONG_SIDE)
                    .await?;
                services.uploader.upload(id.clone(), image).await?;
                Ok::<(), RecordError>(())
            }
            .await;
            match result {
                Ok(()) => {
                    mark_records_changed(&mut revision);
                }
                // 取り消しは失敗ではない。
                Err(RecordError::Photo(error)) if error.message == "cancelled" => {}
                Err(failure) => photo_error.set(Some(failure)),
            }
            photo_busy.set(false);
        });
    });

    let delete_services = services.clone();
    let delete_id = id.clone();
    let delete_photo = EventHandler::new(move |_| {
        let api = RecordsApi::new(delete_services.api.clone());
        let id = delete_id.clone();
        spawn(async move {
            photo_busy.set(true);
            photo_error.set(None);
            match api.delete_photo(&id).await {
                Ok(_) => {
                    mark_records_changed(&mut revision);
                }
                Err(failure) => photo_error.set(Some(failure)),
            }
            photo_busy.set(false);
        });
    });

    // 削除 (0056)。削除の影響の件数を引いて確認を出し、失敗は再試行のバナーで知らせる。
    let request_services = services.clone();
    let request_id = id.clone();
    let mut request_revision = revision;
    let mut request_notice = notice;
    let close_after_delete = EventHandler::new(move |_| match on_deleted {
        Some(handler) => handler.call(()),
        None => {
            navigator.go_back();
        }
    });
    let request_delete = EventHandler::new(move |_| {
        let api = RecordsApi::new(request_services.api.clone());
        let id = request_id.clone();
        spawn(async move {
            match api.purchase_delete_impact(&id).await {
                Ok(impact) => {
                    delete_impact.set(Some(impact));
                    delete_open.set(true);
                }
                // 記録が既に無い場合は成功と同じ扱いにする (0056)。
                Err(failure) if record_error_not_found(&failure) => {
                    mark_records_changed(&mut request_revision);
                    request_notice.set(Some(t(Key::RecordDeletedMessage).to_string()));
                    close_after_delete.call(());
                }
                Err(failure) => impact_error.set(Some(failure)),
            }
        });
    });
    let record_services = services.clone();
    let record_id = id.clone();
    let mut record_revision = revision;
    let mut record_notice = notice;
    let delete_now = EventHandler::new(move |_| {
        let api = RecordsApi::new(record_services.api.clone());
        let id = record_id.clone();
        spawn(async move {
            match api.delete_purchase(&id).await {
                Ok(()) => {}
                // 記録が既に無い場合は成功と同じ扱いにする (0056)。
                Err(failure) if record_error_not_found(&failure) => {}
                Err(failure) => {
                    delete_error.set(Some(failure));
                    return;
                }
            }
            mark_records_changed(&mut record_revision);
            record_notice.set(Some(t(Key::RecordDeletedMessage).to_string()));
            close_after_delete.call(());
        });
    });

    let language = current_language();
    let current = purchase();
    let failure = error();
    let ratings_failure = ratings_error();
    let taste_profile = taste();
    let taste_failure = taste_error();
    let photo_failure = photo_error();
    let impact_failure = impact_error();
    let delete_failure = delete_error();
    let delete_message = delete_impact().map(|impact| {
        if impact.brews > 0 {
            t_args(
                Key::DeletePurchaseConfirmMessageWithBrews,
                &[("count", &impact.brews.to_string())],
            )
        } else {
            t(Key::DeletePurchaseConfirmMessage).to_string()
        }
    });
    let photo_url = format!("{}/purchases/{}/photo", services.api.base_path(), id);
    let favorited = current
        .as_ref()
        .is_some_and(|purchase| purchase.favorited_at.is_some());
    let favorite_services = services.clone();
    let favorite_id = id.clone();
    let mut favorite_revision = revision;
    let mut favorite_notice = notice;
    let favorite = EventHandler::new(move |_| {
        let api = RecordsApi::new(favorite_services.api.clone());
        let id = favorite_id.clone();
        spawn(async move {
            match api.set_purchase_favorite(&id, !favorited).await {
                Ok(_) => mark_records_changed(&mut favorite_revision),
                Err(failure) => {
                    favorite_notice.set(Some(t(record_error_key(&failure)).to_string()))
                }
            }
        });
    });
    let duplicate_id = id.clone();
    let actions = current.as_ref().map(|_| {
        let favorite_name = if favorited { "star" } else { "star_border" }.to_string();
        let favorite_label = t(if favorited {
            Key::FavoriteRemoveLabel
        } else {
            Key::FavoriteAddLabel
        })
        .to_string();
        rsx! {
            IconButton {
                name: favorite_name,
                label: favorite_label,
                onclick: move |_| favorite.call(()),
            }
            IconButton {
                name: "content_copy".to_string(),
                label: t(Key::DuplicatePurchaseLabel).to_string(),
                onclick: move |_| match on_duplicate {
                    Some(handler) => handler.call(()),
                    None => {
                        let _ = navigator.push(Route::PurchaseNew {
                            from: Some(duplicate_id.clone()),
                        });
                    }
                },
            }
            IconButton {
                name: "edit".to_string(),
                label: t(Key::EditButton).to_string(),
                onclick: move |_| match on_edit {
                    Some(handler) => handler.call(()),
                    None => {
                        let _ = navigator.push(Route::PurchaseEdit { id: id.clone() });
                    }
                },
            }
            IconButton {
                name: "delete".to_string(),
                label: t(Key::DeleteButton).to_string(),
                onclick: move |_| request_delete.call(()),
            }
        }
    });

    rsx! {
        div { class: "screen",
            ScreenAppBar {
                title: t(Key::PurchaseDetailTitle).to_string(),
                menu: false,
                leading_icon: (!embedded).then_some("arrow_back_ios_new".to_string()),
                leading_label: Some(t(Key::CancelButton).to_string()),
                on_leading: move |_| {
                    navigator.go_back();
                },
                actions,
            }
            div { class: "body",
                if let Some(purchase) = current {
                    div { class: "detail",
                        div { class: "detail-head",
                            div { class: "photo",
                                style: "width: 112px; height: 112px;",
                                if purchase.photo_key.is_some() {
                                    img { src: photo_url.clone(), alt: t(Key::PhotoLabel).to_string() }
                                } else {
                                    crate::ui::Icon { name: "photo_camera".to_string(), muted: true }
                                }
                            }
                            div { class: "detail-head-main",
                                div { class: "name", "{purchase.name}" }
                                if let Some(shop) = purchase.shop.as_ref() {
                                    div { class: "t-caption muted", "{shop.name}" }
                                }
                                if !purchase.flavor_notes.is_empty() {
                                    div { class: "chips",
                                        for note in purchase.flavor_notes.clone() {
                                            TagChip { label: note }
                                        }
                                    }
                                }
                            }
                        }
                        Ledger {
                            LedgerRow {
                                label: t(Key::Producer).to_string(),
                                value: purchase.producer.clone(),
                                mono: false,
                            }
                            LedgerRow {
                                label: t(Key::Origin).to_string(),
                                value: purchase.origin.clone(),
                                mono: false,
                            }
                            LedgerRow {
                                label: t(Key::Region).to_string(),
                                value: purchase.region.clone(),
                                mono: false,
                            }
                            LedgerRow {
                                label: t(Key::Process).to_string(),
                                value: purchase.process.clone(),
                                mono: false,
                            }
                            LedgerRow {
                                label: t(Key::Variety).to_string(),
                                value: purchase.variety.clone(),
                                mono: false,
                            }
                            LedgerRow {
                                label: t(Key::PurchasedOnLabel).to_string(),
                                value: Some(purchase_row_subtitle(&purchase, language)),
                            }
                            LedgerRow {
                                label: t(Key::Roast).to_string(),
                                value: purchase.roast.clone(),
                                mono: false,
                            }
                            LedgerRow {
                                label: t(Key::RoastDate).to_string(),
                                value: purchase.roast_date.clone(),
                            }
                            LedgerRow {
                                label: t(Key::PriceLabel).to_string(),
                                value: purchase.price_amount.map(|amount| amount.to_string()),
                                unit: purchase.price_currency.clone(),
                            }
                            LedgerRow {
                                label: t(Key::WeightLabel).to_string(),
                                value: purchase.weight_grams.map(|value| value.to_string()),
                                unit: Some(t(Key::GramUnit).to_string()),
                            }
                            LedgerRow {
                                label: t(Key::PhotoLabel).to_string(),
                                trailing: Some(rsx! {
                                    div { class: "photo-actions",
                                        if photo_busy() {
                                            span { class: "t-body faint", "{t(Key::UploadingLabel)}" }
                                        } else {
                                            Button {
                                                label: t(if purchase.photo_key.is_some() { Key::PhotoReplaceButton } else { Key::PhotoSelectButton }).to_string(),
                                                variant: ButtonVariant::Text,
                                                onclick: move |_| replace_photo.call(()),
                                            }
                                            if purchase.photo_key.is_some() {
                                                Button {
                                                    label: t(Key::PhotoDeleteButton).to_string(),
                                                    variant: ButtonVariant::Text,
                                                    onclick: move |_| delete_photo.call(()),
                                                }
                                            }
                                        }
                                    }
                                }),
                            }
                        }
                        div { class: "section taste-profile",
                            h2 { "{t(Key::TasteProfileTitle)}" }
                            div { class: "t-caption muted", "{t(Key::TasteProfileDescription)}" }
                            div { class: "acts",
                                Button {
                                    label: t(if taste_profile.is_some() { Key::TasteProfileRefreshButton } else { Key::TasteProfileButton }).to_string(),
                                    variant: ButtonVariant::Secondary,
                                    size: ButtonSize::Sm,
                                    icon: Some("auto_awesome".to_string()),
                                    disabled: taste_busy(),
                                    onclick: move |_| load_taste.call(()),
                                }
                            }
                            if taste_busy() {
                                div { class: "t-caption muted", "{t(Key::TasteProfileLoading)}" }
                            }
                            if let Some(profile) = taste_profile {
                                if !profile.summary.is_empty() {
                                    div { class: "t-body taste-summary", "{profile.summary}" }
                                }
                                if !profile.flavor_notes.is_empty() {
                                    div { class: "chips taste-notes",
                                        for note in profile.flavor_notes.clone() {
                                            TagChip { label: note }
                                        }
                                    }
                                }
                                Ledger {
                                    if let Some(value) = profile.acidity.clone() {
                                        LedgerRow {
                                            label: t(Key::TasteAcidity).to_string(),
                                            value: Some(value),
                                            mono: false,
                                        }
                                    }
                                    if let Some(value) = profile.sweetness.clone() {
                                        LedgerRow {
                                            label: t(Key::TasteSweetness).to_string(),
                                            value: Some(value),
                                            mono: false,
                                        }
                                    }
                                    if let Some(value) = profile.bitterness.clone() {
                                        LedgerRow {
                                            label: t(Key::TasteBitterness).to_string(),
                                            value: Some(value),
                                            mono: false,
                                        }
                                    }
                                    if let Some(value) = profile.body.clone() {
                                        LedgerRow {
                                            label: t(Key::TasteBody).to_string(),
                                            value: Some(value),
                                            mono: false,
                                        }
                                    }
                                    if let Some(value) = profile.aroma.clone() {
                                        LedgerRow {
                                            label: t(Key::TasteAroma).to_string(),
                                            value: Some(value),
                                            mono: false,
                                        }
                                    }
                                    if let Some(value) = profile.process_description.clone() {
                                        LedgerRow {
                                            label: t(Key::TasteProcessDescription).to_string(),
                                            value: Some(value),
                                            mono: false,
                                        }
                                    }
                                    if let Some(value) = profile.region_characteristics.clone() {
                                        LedgerRow {
                                            label: t(Key::TasteRegionCharacteristics).to_string(),
                                            value: Some(value),
                                            mono: false,
                                        }
                                    }
                                    if let Some(value) = profile.brewing_method.clone() {
                                        LedgerRow {
                                            label: t(Key::TasteBrewingMethod).to_string(),
                                            value: Some(value),
                                            mono: false,
                                        }
                                    }
                                    if let Some(value) = profile.grind_setting.clone() {
                                        LedgerRow {
                                            label: t(Key::TasteGrindSetting).to_string(),
                                            value: Some(value),
                                            mono: false,
                                        }
                                    }
                                }
                                div { class: "t-caption faint", "{t(Key::TasteProfileDisclaimer)}" }
                            }
                            if let Some(failure) = taste_failure {
                                Banner {
                                    message: t(Key::TasteProfileFailed).to_string(),
                                    on_retry: record_error_retry(&failure).then_some(EventHandler::new(move |_| load_taste.call(()))),
                                }
                            }
                        }
                        RatingHistoryChart { entries: ratings() }
                        ReferenceChain {
                            for (kind, name) in purchase_reference_tiles(&purchase).into_iter() {
                                {
                                    let route = purchase.shop.as_ref().map(|shop| Route::ShopEdit {
                                        id: shop.id.clone(),
                                    });
                                    rsx! {
                                        ReferenceTile {
                                            kind: t(kind).to_string(),
                                            name,
                                            on_click: route.map(|route| EventHandler::new(move |_| {
                                                let _ = navigator.push(route.clone());
                                            })),
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "t-caption muted",
                            {purchase_tile_name(&purchase, language)}
                        }
                    }
                } else if loading() {
                    div { class: "empty", "{t(Key::Loading)}" }
                }
                if let Some(failure) = photo_failure {
                    {retryable_banner(&failure, replace_photo)}
                }
                if let Some(failure) = ratings_failure {
                    {retryable_banner(&failure, retry_reload)}
                }
                if let Some(failure) = failure {
                    {retryable_banner(&failure, retry_reload)}
                }
                if let Some(failure) = impact_failure {
                    {retryable_banner(&failure, request_delete)}
                }
                if let Some(failure) = delete_failure {
                    {retryable_banner(&failure, delete_now)}
                }
            }
        }
        if delete_open() {
            DeleteConfirm {
                open: delete_open,
                title: t(Key::DeletePurchaseConfirmTitle).to_string(),
                message: delete_message.unwrap_or_default(),
                on_confirm: move |_| delete_now.call(()),
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
