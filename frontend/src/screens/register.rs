//! 登録用トークンによるパスキーの登録の画面 (FR-1)。
//!
//! トークンは `/register?token=<トークン>` のクエリで受け取る。登録が成功するとセッションが
//! 発行され、遷移の判定がホームへ戻す。トークンの 404、409、410 はバナーに文言を出し、
//! フォームを出さずに再発行の案内を続ける (Flutter と同じ。docs/design/components/Auth)。

use dioxus::prelude::*;

use crate::auth::{
    is_invalid_registration_token, passkey_name_for_request, register, register_error_key,
    AuthError, AuthServices, SessionStatus,
};
use crate::i18n::{t, Key};
use crate::ui::{AppBar, Banner, Button, Field, TextField};

/// 送信の前に決めること (0040 のレビューの指摘で切り出した)。
///
/// 名前の検証 (1 文字以上 50 文字以下) とトークンの有無を、API を呼ぶ前に決める。
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RegisterSubmit {
    /// 送信する。トークンと、前後の空白を除いた名前。
    Send { token: String, name: String },

    /// 送信しない。入力欄に出すエラー。
    NameError(Key),

    /// 送信しない。トークンが無い (フォームを出さない)。
    MissingToken,
}

/// 送信の前に決めることを返す。
pub fn register_submit(name: &str, token: Option<&str>) -> RegisterSubmit {
    match passkey_name_for_request(name) {
        Ok(name) => match token.filter(|token| !token.is_empty()) {
            Some(token) => RegisterSubmit::Send {
                token: token.to_string(),
                name,
            },
            None => RegisterSubmit::MissingToken,
        },
        Err(key) => RegisterSubmit::NameError(key),
    }
}

/// 登録用トークンによるパスキーの登録の画面 (FR-1)。
#[component]
pub fn RegisterScreen(token: Option<String>) -> Element {
    let services = use_context::<AuthServices>();
    let mut session = use_context::<Signal<SessionStatus>>();
    let mut name = use_signal(String::new);
    let mut name_error = use_signal(|| None::<Key>);
    let mut failure = use_signal(|| None::<AuthError>);
    let mut busy = use_signal(|| false);

    let submit_token = token.clone();
    let submit = EventHandler::new(move |_| {
        if busy() {
            return;
        }
        let (token, request_name) = match register_submit(&name(), submit_token.as_deref()) {
            RegisterSubmit::Send { token, name } => (token, name),
            RegisterSubmit::NameError(key) => {
                name_error.set(Some(key));
                return;
            }
            RegisterSubmit::MissingToken => return,
        };
        busy.set(true);
        name_error.set(None);
        failure.set(None);
        let services = services.clone();
        spawn(async move {
            match register(&services, &token, &request_name).await {
                Ok(()) => session.set(SessionStatus::SignedIn),
                Err(error) => failure.set(Some(error)),
            }
            busy.set(false);
        });
    });

    // トークンが無いときと、API が 404、409、410 を返したときはフォームを出さない (Flutter と同じ)。
    let token_error_key = if token.as_deref().is_none_or(str::is_empty) {
        Some(Key::RegisterTokenMissing)
    } else {
        failure()
            .as_ref()
            .filter(|error| is_invalid_registration_token(error))
            .map(register_error_key)
    };

    rsx! {
        AppBar { title: t(Key::RegisterTitle).to_string() }
        if let Some(key) = token_error_key {
            div { class: "form",
                Banner { message: t(key).to_string() }
                p { class: "t-body muted", "{t(Key::RegisterTokenGuidance)}" }
            }
        } else {
            div { class: "form",
                p { class: "t-body muted", "{t(Key::RegisterDescription)}" }
                Field {
                    label: t(Key::PasskeyNameLabel).to_string(),
                    required: true,
                    error: name_error().map(|key| t(key).to_string()),
                    help: Some(t(Key::PasskeyNameHelper).to_string()),
                    disabled: busy(),
                    TextField {
                        value: name(),
                        placeholder: Some(t(Key::PasskeyNameHint).to_string()),
                        disabled: busy(),
                        oninput: move |event: FormEvent| name.set(event.value()),
                    }
                }
                Button {
                    label: t(Key::RegisterButton).to_string(),
                    block: true,
                    disabled: busy(),
                    onclick: submit,
                }
                if let Some(error) = failure() {
                    Banner { message: t(register_error_key(&error)).to_string() }
                }
            }
        }
    }
}
