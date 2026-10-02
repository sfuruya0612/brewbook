//! ログインの画面 (FR-2)。
//!
//! パスキーだけでログインするため、利用者名とパスワードの入力は置かない (ADR-0004)。画面の
//! 中央に印と名前と説明と主要ボタンを置き、下端に登録用リンクの案内を出す (AppBar は無い。
//! docs/design/components/Auth)。

use dioxus::prelude::*;

use crate::auth::{login, login_error_key, AuthError, AuthServices, SessionStatus};
use crate::i18n::{t, Key};
use crate::ui::{Banner, BrewbookMark, Button};

/// ログインの画面 (FR-2)。
#[component]
pub fn LoginScreen() -> Element {
    let services = use_context::<AuthServices>();
    let mut session = use_context::<Signal<SessionStatus>>();
    let mut busy = use_signal(|| false);
    let mut failure = use_signal(|| None::<AuthError>);

    let submit = EventHandler::new(move |_| {
        if busy() {
            return;
        }
        busy.set(true);
        failure.set(None);
        let services = services.clone();
        spawn(async move {
            match login(&services).await {
                Ok(()) => session.set(SessionStatus::SignedIn),
                Err(error) => failure.set(Some(error)),
            }
            busy.set(false);
        });
    });

    rsx! {
        div { class: "min-h-dvh flex flex-col px-6",
            div { class: "flex-1 flex flex-col justify-center gap-4",
                div { class: "mx-auto mb-2",
                    BrewbookMark { size: 72 }
                }
                h1 { class: "t-wordmark text-center", "{t(Key::HomeTitle)}" }
                p { class: "t-body muted text-center mt-4", "{t(Key::LoginDescription)}" }
                div { class: "mt-8",
                    Button {
                        label: t(Key::LoginButton).to_string(),
                        block: true,
                        icon: Some("key".to_string()),
                        disabled: busy(),
                        onclick: submit,
                    }
                }
                if let Some(error) = failure() {
                    Banner { message: t(login_error_key(&error)).to_string() }
                }
            }
            div { class: "t-caption muted text-center pt-4 pb-6", "{t(Key::LoginRegisterHint)}" }
        }
    }
}
