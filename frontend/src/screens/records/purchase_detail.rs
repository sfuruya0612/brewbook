//! 購入の詳細の画面 (FR-9、FR-10、UC-6)。
//!
//! 商品と店をたどれるようにし、写真の差し替えと削除ができる (FR-10)。幅 840 px 以上の
//! 2 段組では `embedded` を true にし、右の面に出す。

use dioxus::prelude::*;
use dioxus_router::navigator;

use crate::i18n::{current_language, t, t_args, Key};
use crate::records::display::{
    purchase_reference_tiles, purchase_row_subtitle, purchase_tile_name,
};
use crate::records::stats::RatingHistoryEntry;
use crate::records::{
    record_error_key, record_error_not_found, DeleteImpact, RecordError, RecordServices,
    RecordsApi, StatsApi, MAX_PHOTO_LONG_SIDE,
};
use crate::router::Route;
use crate::screens::stats::RatingHistoryChart;
use crate::screens::ScreenAppBar;
use crate::ui::{
    Button, ButtonVariant, IconButton, Ledger, LedgerRow, ReferenceChain, ReferenceTile, TagChip,
};

use super::{clear_notice_after, mark_records_changed, retryable_banner, DeleteConfirm};

/// 購入の詳細 (FR-9)。
#[component]
pub fn PurchaseDetailScreen(id: String) -> Element {
    rsx! {
        PurchaseDetail { id }
    }
}

/// 購入の詳細の中身。2 段組の右の面にも出せる。
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
    let mut photo_error = use_signal(|| None::<RecordError>);
    let mut photo_busy = use_signal(|| false);
    let mut loading = use_signal(|| true);
    // 削除の確認と、削除の影響と失敗の再試行 (0056)。
    let mut delete_open = use_signal(|| false);
    let mut delete_impact = use_signal(|| None::<DeleteImpact>);
    let mut impact_error = use_signal(|| None::<RecordError>);
    let mut delete_error = use_signal(|| None::<RecordError>);

    let reload_services = services.clone();
    let reload_id = id.clone();
    let reload = EventHandler::new(move |_| {
        let api = RecordsApi::new(reload_services.api.clone());
        let stats = StatsApi::new(reload_services.api.clone());
        let id = reload_id.clone();
        spawn(async move {
            loading.set(true);
            error.set(None);
            ratings_error.set(None);
            match api.purchase(&id).await {
                Ok(loaded) => purchase.set(Some(loaded)),
                Err(failure) => error.set(Some(failure)),
            }
            // 評価の推移の失敗は、購入の表示を残したままグラフの区画にだけ出す (FR-18)。
            match stats.rating_history(&id).await {
                Ok(loaded) => ratings.set(loaded),
                Err(failure) => ratings_error.set(Some(failure)),
            }
            loading.set(false);
        });
    });
    // マウント時と、記録が変わったときに読み直す。
    use_effect(move || {
        let _ = revision();
        reload.call(());
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
                                div { class: "name", "{purchase.product.name}" }
                                if let Some(shop) = purchase.shop.as_ref() {
                                    div { class: "t-caption muted", "{shop.name}" }
                                }
                                if !purchase.product.flavor_notes.is_empty() {
                                    div { class: "chips",
                                        for note in purchase.product.flavor_notes.clone() {
                                            TagChip { label: note }
                                        }
                                    }
                                }
                            }
                        }
                        Ledger {
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
                        RatingHistoryChart { entries: ratings() }
                        ReferenceChain {
                            for (kind, name) in purchase_reference_tiles(&purchase).into_iter() {
                                {
                                    let route = match kind {
                                        Key::ProductLabel => Some(Route::ProductEdit {
                                            id: purchase.product.id.clone(),
                                        }),
                                        _ => purchase.shop.as_ref().map(|shop| Route::ShopEdit {
                                            id: shop.id.clone(),
                                        }),
                                    };
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
                    {retryable_banner(&failure, reload)}
                }
                if let Some(failure) = failure {
                    {retryable_banner(&failure, reload)}
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
