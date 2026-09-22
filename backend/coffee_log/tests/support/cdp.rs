//! Chrome を CDP で起動し、仮想認証器を付けたページを操作するハーネス (ADR-0004)。
//!
//! `WebAuthn.enable`、`WebAuthn.addVirtualAuthenticator`、`Runtime.evaluate` を送れることは
//! 実装の最初に確認済み (issue 0005)。テストはテストページの `window.coffeeLogTest` を呼び、
//! `navigator.credentials` でパスキーを作る・使う。
//!
//! 署名カウンタの後退の検査のため、仮想認証器のクレデンシャルを入れ替える
//! (`WebAuthn.getCredentials` と `WebAuthn.removeCredential` と `WebAuthn.addCredential`)。
//! Chrome の仮想認証器は操作のたびにカウンタを増やすため、保存値より小さいカウンタを
//! 入れ直すことで後退を作れる。

use std::sync::Arc;
use std::time::Duration;

use headless_chrome::protocol::cdp::WebAuthn;
use headless_chrome::{Browser, LaunchOptions, Tab};
use serde_json::Value;

/// Chrome と仮想認証器を付けたページ。
pub struct TestBrowser {
    /// Chrome の子プロセスを生かしておくために持つ。
    _browser: Browser,
    tab: Arc<Tab>,
    authenticator_id: String,
}

impl TestBrowser {
    /// Chrome を起動し、`url` を開き、仮想認証器を 1 つ付ける。
    pub fn open(url: &str) -> Result<Self, String> {
        let browser = Browser::new(LaunchOptions {
            // CI のコンテナでも起動できるように sandbox を切る。
            sandbox: false,
            // 共有メモリの小さい環境 (コンテナ) でも落ちないようにする。
            args: vec![std::ffi::OsStr::new("--disable-dev-shm-usage")],
            // テストの待ち時間で Chrome を落とさないように長くする。
            idle_browser_timeout: Duration::from_secs(600),
            ..LaunchOptions::default()
        })
        .map_err(|error| format!("Chrome must launch: {error}"))?;
        let tab = browser
            .new_tab()
            .map_err(|error| format!("a tab must open: {error}"))?;
        tab.navigate_to(url)
            .map_err(|error| format!("the test page must load: {error}"))?;
        tab.wait_until_navigated()
            .map_err(|error| format!("the test page must navigate: {error}"))?;
        tab.call_method(WebAuthn::Enable { enable_ui: None })
            .map_err(|error| format!("WebAuthn.enable must be sent: {error}"))?;
        // CTAP2 の仮想認証器を内部認証器として付ける。discoverable なクレデンシャルを作り
        // (ログインの allowCredentials を空にするため)、利用者の確認に常に成功する。
        let authenticator = tab
            .call_method(WebAuthn::AddVirtualAuthenticator {
                options: WebAuthn::VirtualAuthenticatorOptions {
                    protocol: WebAuthn::AuthenticatorProtocol::Ctap2,
                    ctap_2_version: None,
                    transport: WebAuthn::AuthenticatorTransport::Internal,
                    has_resident_key: Some(true),
                    has_user_verification: Some(true),
                    has_large_blob: None,
                    has_cred_blob: None,
                    has_min_pin_length: None,
                    has_prf: None,
                    automatic_presence_simulation: Some(true),
                    is_user_verified: Some(true),
                    default_backup_eligibility: None,
                    default_backup_state: None,
                },
            })
            .map_err(|error| format!("WebAuthn.addVirtualAuthenticator must be sent: {error}"))?;
        Ok(Self {
            _browser: browser,
            tab,
            authenticator_id: authenticator.authenticator_id,
        })
    }

    /// ページの式を評価し、返った値を JSON にする。式は値でも Promise でもよい。
    /// 式は `window.coffeeLogTest` の関数を `await` で呼ぶ形にする。
    pub fn evaluate_json(&self, expression: &str) -> Result<Value, String> {
        // 式が Promise の場合は解決した値を待つ (await を付けないと Promise が JSON になる)。
        let script = format!("(async () => JSON.stringify(await ({expression})))()");
        let result = self
            .tab
            .evaluate(&script, true)
            .map_err(|error| format!("Runtime.evaluate must run: {error}"))?;
        let Some(value) = result.value else {
            return Err(format!(
                "the script must return a value but returned {result:?}"
            ));
        };
        let text = value
            .as_str()
            .ok_or_else(|| format!("the script must return a JSON string but returned {value}"))?;
        serde_json::from_str(text)
            .map_err(|error| format!("the script must return JSON but returned {text}: {error}"))
    }

    /// 仮想認証器のクレデンシャルの署名カウンタを付け替える。クレデンシャルが無ければ失敗する。
    pub fn set_sign_count(&self, credential_id: &str, sign_count: u32) -> Result<(), String> {
        let credentials = self
            .tab
            .call_method(WebAuthn::GetCredentials {
                authenticator_id: self.authenticator_id.clone(),
            })
            .map_err(|error| format!("WebAuthn.getCredentials must be sent: {error}"))?;
        let wanted = canonical_credential_id(credential_id);
        let mut credential = credentials
            .credentials
            .into_iter()
            .find(|credential| canonical_credential_id(&credential.credential_id) == wanted)
            .ok_or_else(|| {
                format!("the virtual authenticator has no credential {credential_id}")
            })?;
        credential.sign_count = sign_count;
        self.tab
            .call_method(WebAuthn::RemoveCredential {
                authenticator_id: self.authenticator_id.clone(),
                credential_id: credential.credential_id.clone(),
            })
            .map_err(|error| format!("WebAuthn.removeCredential must be sent: {error}"))?;
        self.tab
            .call_method(WebAuthn::AddCredential {
                authenticator_id: self.authenticator_id.clone(),
                credential,
            })
            .map_err(|error| format!("WebAuthn.addCredential must be sent: {error}"))?;
        Ok(())
    }
}

/// クレデンシャル ID を同じ形にする。
/// CDP は標準の base64 (パディングあり) で返し、API は base64url (パディングなし) で返す。
fn canonical_credential_id(id: &str) -> String {
    let normalized = id.replace('+', "-").replace('/', "_");
    let trimmed = normalized.trim_end_matches('=');
    match coffee_log_core::base64url::decode(trimmed) {
        Ok(bytes) => coffee_log_core::base64url::encode(&bytes),
        Err(_) => id.to_owned(),
    }
}

/// JS の文字列リテラルにする。
pub fn js_string(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}
