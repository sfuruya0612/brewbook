//! 設定の画面 (FR-3、FR-4、FR-14、FR-15)。
//!
//! パスキーの管理、全記録のエクスポート、アカウントと全データの削除、ログアウトを 1 つの
//! 画面にまとめる (docs/design/components/Settings)。削除は確認のダイアログ (Feedback) を
//! 経て行い、確認のボタンを押したときだけ API を呼ぶ (FR-15)。

use dioxus::prelude::*;
use dioxus_router::navigator;

use crate::auth::{
    add_passkey, delete_account, delete_passkey, delete_passkey_error_key, logout, message_key,
    passkey_name_for_request, passkeys, rename_passkey, AuthError, AuthServices, Passkey,
    SessionStatus,
};
use crate::i18n::{current_language, t, text, text_args, Key, Language};
use crate::records::values::{display_timestamp, parse_utc_to_local};
use crate::records::RecordServices;
use crate::screens::records::{clear_notice_after, rail_items};
use crate::settings::{export_all, SettingsServices};
use crate::ui::{
    AppBar, Banner, Button, ButtonSize, ButtonVariant, ConfirmDialog, Field, Icon, IconButton,
    NavigationRail, Snackbar, TextField, WidePage,
};

/// パスキーの名前を入力するダイアログの種類 (FR-3)。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PasskeyDialog {
    /// 追加。名前は空から始める。
    Add,

    /// 名前の変更。現在の名前を初期値にする。
    Rename {
        /// 変更するパスキーの ID。
        id: String,
    },
}

/// アカウント削除の確認の状態 (FR-15)。
///
/// 入口のボタンで確認を出し、確認のボタンを押したときだけ [`AccountDelete::confirm`] が true を
/// 返す。画面は true のときだけ削除の API を呼ぶ。取り消しと、確認を出していないときの確認は
/// false のままにする。
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct AccountDelete {
    /// 確認を出しているか。
    open: bool,
}

impl AccountDelete {
    /// 確認を出していない状態から作る。
    pub const fn new() -> Self {
        Self { open: false }
    }

    /// 入口のボタンを押した。確認を出す。
    pub fn request(&mut self) {
        self.open = true;
    }

    /// 確認を出しているか。
    pub fn is_open(self) -> bool {
        self.open
    }

    /// 取り消した。API は呼ばない。
    pub fn cancel(&mut self) {
        self.open = false;
    }

    /// 確認のボタンを押した。true のときだけ画面が API を呼ぶ。
    pub fn confirm(&mut self) -> bool {
        let open = self.open;
        self.open = false;
        open
    }
}

/// パスキーの削除のボタンを押せるか (FR-3)。最後の 1 つは削除できない。
pub fn can_delete_passkey(count: usize) -> bool {
    count > 1
}

/// パスキーの行の補足 (登録日時と最終使用日時。FR-3、FR-16)。
///
/// 日時は端末のタイムゾーンに直し、言語に合わせて表示する。まだ使われていないパスキーは
/// その旨を出す。読めない日時はそのまま返す。
pub fn passkey_subtitle(passkey: &Passkey, language: Language, utc_offset_minutes: i32) -> String {
    let created = text_args(
        language,
        Key::PasskeyCreatedAt,
        &[(
            "timestamp",
            &display_timestamp_text(&passkey.created_at, language, utc_offset_minutes),
        )],
    );
    let last_used = match passkey.last_used_at.as_deref() {
        Some(timestamp) => text_args(
            language,
            Key::PasskeyLastUsedAt,
            &[(
                "timestamp",
                &display_timestamp_text(timestamp, language, utc_offset_minutes),
            )],
        ),
        None => text(language, Key::PasskeyNotUsedYet).to_string(),
    };
    format!("{created} / {last_used}")
}

/// UTC の日時を端末のタイムゾーンと言語で表示する (FR-16)。読めない日時はそのまま返す。
fn display_timestamp_text(timestamp: &str, language: Language, utc_offset_minutes: i32) -> String {
    match parse_utc_to_local(timestamp, utc_offset_minutes) {
        Some(datetime) => display_timestamp(datetime, language),
        None => timestamp.to_string(),
    }
}

/// 設定の画面 (FR-3、FR-4、FR-14、FR-15)。
#[component]
pub fn SettingsScreen() -> Element {
    let auth = use_context::<AuthServices>();
    let settings = use_context::<SettingsServices>();
    let records = use_context::<RecordServices>();
    let mut session = use_context::<Signal<SessionStatus>>();
    let mut notice = use_context::<Signal<Option<String>>>();
    let navigator = navigator();
    let offset = records.clock.utc_offset_minutes();
    let language = current_language();

    // パスキーの一覧 (FR-3)。読み込みが終わるまでは None。
    let mut passkey_list = use_signal(|| None::<Vec<Passkey>>);
    let mut passkeys_error = use_signal(|| None::<AuthError>);
    // 進行中の操作。二重の実行を防ぐため、操作の間はボタンを無効にする。
    let mut busy = use_signal(|| false);
    // 名前を入力するダイアログと、その入力 (FR-3)。
    let mut dialog = use_signal(|| None::<PasskeyDialog>);
    let mut dialog_name = use_signal(String::new);
    let mut dialog_error = use_signal(|| None::<Key>);
    // アカウント削除の確認 (FR-15)。
    let mut account_delete = use_signal(AccountDelete::new);
    // パスキーの一覧を読み直す合図。
    let mut revision = use_signal(|| 0_u64);

    // マウント時と、パスキーが変わったときに一覧を読み直す (revision の読み取りで再実行される)。
    let load_auth = auth.clone();
    use_effect(move || {
        let _ = revision();
        let auth = load_auth.clone();
        spawn(async move {
            match passkeys(&auth).await {
                Ok(values) => {
                    passkey_list.set(Some(values));
                    passkeys_error.set(None);
                }
                Err(error) => passkeys_error.set(Some(error)),
            }
        });
    });

    let retry = EventHandler::new(move |_| revision.set(revision() + 1));

    let open_add = EventHandler::new(move |_| {
        dialog_name.set(String::new());
        dialog_error.set(None);
        dialog.set(Some(PasskeyDialog::Add));
    });

    let open_rename = EventHandler::new(move |passkey: Passkey| {
        dialog_name.set(passkey.name.clone());
        dialog_error.set(None);
        dialog.set(Some(PasskeyDialog::Rename { id: passkey.id }));
    });

    let cancel_dialog = EventHandler::new(move |_| {
        dialog.set(None);
        dialog_error.set(None);
    });

    let dialog_auth = auth.clone();
    let confirm_dialog = EventHandler::new(move |_| {
        let Some(current) = dialog() else {
            return;
        };
        let name = match passkey_name_for_request(&dialog_name()) {
            Ok(name) => name,
            Err(key) => {
                dialog_error.set(Some(key));
                return;
            }
        };
        dialog.set(None);
        dialog_error.set(None);
        busy.set(true);
        let auth = dialog_auth.clone();
        spawn(async move {
            let result = match current {
                PasskeyDialog::Add => add_passkey(&auth, &name)
                    .await
                    .map(|_| Key::PasskeyAddedMessage),
                PasskeyDialog::Rename { id } => rename_passkey(&auth, &id, &name)
                    .await
                    .map(|_| Key::PasskeyRenamedMessage),
            };
            match result {
                Ok(message) => {
                    notice.set(Some(t(message).to_string()));
                    revision.set(revision() + 1);
                }
                Err(error) => notice.set(Some(t(message_key(&error)).to_string())),
            }
            busy.set(false);
        });
    });

    let delete_auth = auth.clone();
    let delete = EventHandler::new(move |id: String| {
        if busy() {
            return;
        }
        // 最後の 1 つは削除しない (FR-3)。サーバーも 409 を返す。
        let count = passkey_list().as_ref().map_or(0, Vec::len);
        if !can_delete_passkey(count) {
            return;
        }
        busy.set(true);
        let auth = delete_auth.clone();
        spawn(async move {
            match delete_passkey(&auth, &id).await {
                Ok(()) => {
                    notice.set(Some(t(Key::PasskeyDeletedMessage).to_string()));
                    revision.set(revision() + 1);
                }
                Err(error) => {
                    notice.set(Some(t(delete_passkey_error_key(&error)).to_string()));
                }
            }
            busy.set(false);
        });
    });

    let export = EventHandler::new(move |_| {
        if busy() {
            return;
        }
        busy.set(true);
        let settings = settings.clone();
        spawn(async move {
            match export_all(&settings).await {
                Ok(()) => notice.set(Some(t(Key::ExportDoneMessage).to_string())),
                Err(error) => notice.set(Some(t(message_key(&error)).to_string())),
            }
            busy.set(false);
        });
    });

    let open_delete = EventHandler::new(move |_| account_delete.write().request());

    let cancel_delete = EventHandler::new(move |_| account_delete.write().cancel());

    let delete_account_auth = auth.clone();
    let confirm_delete = EventHandler::new(move |_| {
        // 確認のボタンを押したときだけ API を呼ぶ (FR-15)。
        if !account_delete.write().confirm() || busy() {
            return;
        }
        busy.set(true);
        let auth = delete_account_auth.clone();
        spawn(async move {
            match delete_account(&auth).await {
                Ok(()) => session.set(SessionStatus::SignedOut),
                Err(error) => notice.set(Some(t(message_key(&error)).to_string())),
            }
            busy.set(false);
        });
    });

    let logout_auth = auth.clone();
    let logout_now = EventHandler::new(move |_| {
        if busy() {
            return;
        }
        busy.set(true);
        let auth = logout_auth.clone();
        spawn(async move {
            match logout(&auth).await {
                Ok(()) => session.set(SessionStatus::SignedOut),
                Err(error) => notice.set(Some(t(message_key(&error)).to_string())),
            }
            busy.set(false);
        });
    });

    let failure = passkeys_error();
    let list = passkey_list();
    let can_delete = can_delete_passkey(list.as_ref().map_or(0, Vec::len));
    let passkey_rows = match list {
        Some(values) => rsx! {
            for passkey in values {
                {passkey_row(
                    passkey,
                    can_delete,
                    busy(),
                    language,
                    offset,
                    open_rename,
                    delete,
                )}
            }
        },
        None => rsx! {
            div { class: "empty", "{t(Key::Loading)}" }
        },
    };
    let add_button = passkey_list().is_some().then(|| {
        rsx! {
            div { class: "acts",
                Button {
                    label: t(Key::AddPasskeyButton).to_string(),
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Sm,
                    icon: Some("add".to_string()),
                    disabled: busy(),
                    onclick: open_add,
                }
            }
        }
    });

    let name_dialog = dialog().map(|current| {
        let (title, confirm_label) = match current {
            PasskeyDialog::Add => (Key::AddPasskeyTitle, Key::AddButton),
            PasskeyDialog::Rename { .. } => (Key::RenamePasskeyTitle, Key::RenameButton),
        };
        rsx! {
            PasskeyNameDialog {
                title: t(title).to_string(),
                cancel_label: t(Key::CancelButton).to_string(),
                confirm_label: t(confirm_label).to_string(),
                name: dialog_name(),
                error: dialog_error().map(|key| t(key).to_string()),
                busy: busy(),
                oninput: move |event: FormEvent| dialog_name.set(event.value()),
                on_cancel: move |event| cancel_dialog.call(event),
                on_confirm: move |event| confirm_dialog.call(event),
            }
        }
    });

    rsx! {
        WidePage {
            rail: rsx! { NavigationRail { items: rail_items(navigator, 5) } },
            div { class: "screen",
                AppBar { title: t(Key::SettingsTitle).to_string() }
                div { class: "body",
                    div { class: "section",
                        h2 { "{t(Key::PasskeysTitle)}" }
                        p { "{t(Key::PasskeysDescription)}" }
                        if let Some(error) = failure {
                            Banner {
                                message: t(message_key(&error)).to_string(),
                                on_retry: Some(retry),
                            }
                        } else {
                            {passkey_rows}
                            {add_button}
                        }
                    }
                    div { class: "section",
                        h2 { "{t(Key::ExportTitle)}" }
                        p { "{t(Key::ExportDescription)}" }
                        div { class: "acts",
                            Button {
                                label: t(Key::ExportButton).to_string(),
                                variant: ButtonVariant::Secondary,
                                size: ButtonSize::Sm,
                                icon: Some("download".to_string()),
                                disabled: busy(),
                                onclick: export,
                            }
                        }
                    }
                    div { class: "section",
                        h2 { "{t(Key::DeleteAccountTitle)}" }
                        p { "{t(Key::DeleteAccountDescription)}" }
                        div { class: "acts",
                            Button {
                                label: t(Key::DeleteAccountButton).to_string(),
                                variant: ButtonVariant::DangerOutline,
                                size: ButtonSize::Sm,
                                disabled: busy(),
                                onclick: open_delete,
                            }
                        }
                    }
                    div { class: "section", style: "border-bottom: 0;",
                        div { class: "acts",
                            Button {
                                label: t(Key::LogoutButton).to_string(),
                                variant: ButtonVariant::Text,
                                icon: Some("logout".to_string()),
                                disabled: busy(),
                                onclick: logout_now,
                            }
                        }
                    }
                }
            }
        }
        {name_dialog}
        if account_delete().is_open() {
            ConfirmDialog {
                title: t(Key::DeleteAccountConfirmTitle).to_string(),
                message: t(Key::DeleteAccountConfirmMessage).to_string(),
                cancel_label: t(Key::CancelButton).to_string(),
                confirm_label: t(Key::DeleteConfirmButton).to_string(),
                danger: true,
                on_cancel: move |event| cancel_delete.call(event),
                on_confirm: move |event| confirm_delete.call(event),
            }
        }
        if let Some(message) = notice() {
            {clear_notice_after(notice)}
            div { class: "notice",
                Snackbar { message }
            }
        }
    }
}

/// パスキーの 1 行 (鍵の印、名前、登録日時と最終使用日時、名前の変更と削除。FR-3)。
fn passkey_row(
    passkey: Passkey,
    can_delete: bool,
    busy: bool,
    language: Language,
    utc_offset_minutes: i32,
    on_rename: EventHandler<Passkey>,
    on_delete: EventHandler<String>,
) -> Element {
    let subtitle = passkey_subtitle(&passkey, language, utc_offset_minutes);
    let rename_value = passkey.clone();
    let id = passkey.id.clone();
    rsx! {
        div { class: "pk",
            span { class: "iconbtn", style: "width: 32px; height: 32px; color: var(--ink-muted);",
                Icon { name: "key".to_string(), muted: true }
            }
            div { class: "main",
                div { class: "n", "{passkey.name}" }
                div { class: "m", "{subtitle}" }
            }
            IconButton {
                name: "edit".to_string(),
                label: t(Key::RenameButton).to_string(),
                disabled: busy,
                onclick: move |_| on_rename.call(rename_value.clone()),
            }
            IconButton {
                name: "delete".to_string(),
                label: t(Key::DeleteButton).to_string(),
                disabled: busy || !can_delete,
                onclick: move |_| on_delete.call(id.clone()),
            }
        }
    }
}

/// パスキーの名前を入力するダイアログ (FR-3)。
#[component]
fn PasskeyNameDialog(
    /// 題 (ARB から取る)。
    title: String,
    /// 取り消しのボタンの文言 (ARB から取る)。
    cancel_label: String,
    /// 確定のボタンの文言 (ARB から取る)。
    confirm_label: String,
    /// 入力中の名前。
    name: String,
    /// 検証の誤り (ARB から取る)。
    error: Option<String>,
    /// 実行中か。
    busy: bool,
    /// 入力が変わったときの動き。
    oninput: EventHandler<FormEvent>,
    /// 取り消しの動き。
    on_cancel: EventHandler<MouseEvent>,
    /// 確定の動き。
    on_confirm: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "scrim",
            div { class: "dialog", role: "dialog", "aria-modal": "true", "aria-labelledby": "passkey-dialog-title",
                h2 { id: "passkey-dialog-title", "{title}" }
                Field {
                    label: t(Key::PasskeyNameLabel).to_string(),
                    required: true,
                    error,
                    help: Some(t(Key::PasskeyNameHelper).to_string()),
                    disabled: busy,
                    TextField {
                        value: name,
                        placeholder: Some(t(Key::PasskeyNameHint).to_string()),
                        disabled: busy,
                        oninput: move |event| oninput.call(event),
                    }
                }
                div { class: "acts",
                    Button {
                        label: cancel_label,
                        variant: ButtonVariant::Text,
                        onclick: move |event| on_cancel.call(event),
                    }
                    Button {
                        label: confirm_label,
                        disabled: busy,
                        onclick: move |event| on_confirm.call(event),
                    }
                }
            }
        }
    }
}
