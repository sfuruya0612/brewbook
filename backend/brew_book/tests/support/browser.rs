//! ChromeDriver で Chrome を起動し、仮想認証器を付けたページを操作するハーネス。
//!
//! パスキーの結合テスト (`wrangler_passkey_flow.rs`) の `TestBrowser` と、実バックエンドの
//! E2E (`support/e2e.rs` の `E2eBrowser`) が共有する。仮想認証器は ChromeDriver の WebAuthn の
//! 拡張コマンド (W3C WebAuthn Level 3 の WebDriver 拡張) で付ける。
//!
//! 署名カウンタの後退の検査のため、仮想認証器のクレデンシャルを入れ替える
//! (`GET .../credentials`、`DELETE .../credentials/{id}`、`POST .../credential`)。
//! Chrome の仮想認証器は操作のたびにカウンタを増やすため、保存値より小さいカウンタを
//! 入れ直すことで後退を作れる。

use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use thirtyfour::prelude::*;

use super::free_port;

/// chromedriver の待ち受けを待つ上限。
const CHROMEDRIVER_READY_TIMEOUT: Duration = Duration::from_secs(30);
/// 待ちの確認の間隔。
const POLL_INTERVAL: Duration = Duration::from_millis(100);
/// chromedriver の停止の待ち時間 (`DevServer` と同じ)。
const STOP_TIMEOUT: Duration = Duration::from_secs(5);
/// WebDriver の非同期スクリプトの timeout。パスキーの操作は `fetch` と
/// `navigator.credentials` を `await` するため、chromedriver の既定に依存させずに明示する
/// (0052 の教訓)。
pub const SCRIPT_TIMEOUT: Duration = Duration::from_secs(60);

/// 起動した chromedriver。最後の利用者が落ちたときに停止する。
pub struct ChromeDriver {
    child: Mutex<Child>,
    port: u16,
}

impl ChromeDriver {
    /// 待ち受けているポート。
    pub fn port(&self) -> u16 {
        self.port
    }

    /// 子プロセスが属するプロセスグループの ID (停止の検査に使う)。
    pub fn process_group_id(&self) -> u32 {
        self.child
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .id()
    }
}

impl Drop for ChromeDriver {
    fn drop(&mut self) {
        // セッションの終了に失敗しても Chrome が孤児にならないように、プロセスグループごと
        // 停止する (DevServer と同じ。0019)。
        let mut child = self
            .child
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let process_group = child.id() as libc::pid_t;
        unsafe { libc::killpg(process_group, libc::SIGTERM) };
        let deadline = Instant::now() + STOP_TIMEOUT;
        while Instant::now() < deadline {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => std::thread::sleep(POLL_INTERVAL),
                Err(_) => break,
            }
        }
        unsafe { libc::killpg(process_group, libc::SIGKILL) };
        let _ = child.wait();
    }
}

/// テストバイナリごとに 1 つの chromedriver を共有する。最後の借用が落ちると停止する。
///
/// 7 つのテストが並列に走るため、テストごとに起動すると chromedriver と Chrome の数が
/// 増えすぎる。`DevServer` の `shared_server` と同じ Weak の形で 1 つを共有する。
pub fn shared_chromedriver(task_name: &str) -> Result<Arc<ChromeDriver>, String> {
    static SHARED: OnceLock<Mutex<Weak<ChromeDriver>>> = OnceLock::new();
    let slot = SHARED.get_or_init(|| Mutex::new(Weak::new()));
    let mut guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(driver) = guard.upgrade() {
        return Ok(driver);
    }
    let driver = Arc::new(start_chromedriver(task_name)?);
    *guard = Arc::downgrade(&driver);
    Ok(driver)
}

/// chromedriver を空きポートで起動し、待ち受けを始めるまで待つ。
///
/// `task_name` は chromedriver が見つからないときの案内に使う (mise のタスク名)。
pub fn start_chromedriver(task_name: &str) -> Result<ChromeDriver, String> {
    let port = free_port()?;
    let mut command = Command::new("chromedriver");
    command
        .arg(format!("--port={port}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // 孫 (Chrome) まで確実に停止できるように、子を新しいプロセスグループにする (0019)。
    command.process_group(0);
    let child = command.spawn().map_err(|error| {
        format!("chromedriver must start (run this test with `mise run {task_name}`): {error}")
    })?;
    let driver = ChromeDriver {
        child: Mutex::new(child),
        port,
    };
    wait_for_chromedriver(port)?;
    Ok(driver)
}

/// chromedriver が待ち受けを始めるまで待つ。
///
/// 起動の直後は接続できないため、WebDriver のセッションを開く前に `/status` を確認する。
pub fn wait_for_chromedriver(port: u16) -> Result<(), String> {
    let url = format!("http://127.0.0.1:{port}/status");
    // 接続はできるが応答が返らない場合に期限へ進めるように、短い timeout を設定する。
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|error| format!("the HTTP client must build: {error}"))?;
    let deadline = Instant::now() + CHROMEDRIVER_READY_TIMEOUT;
    loop {
        if let Ok(response) = client.get(&url).send() {
            if response.status().is_success() {
                return Ok(());
            }
        }
        if Instant::now() > deadline {
            return Err(format!("chromedriver must listen on port {port}"));
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// ChromeDriver の WebAuthn の拡張コマンドを 1 つ送り、応答の JSON を返す。
pub async fn web_authn_command(
    driver: &WebDriver,
    server_url: &str,
    method: reqwest::Method,
    endpoint: &str,
    body: Option<Value>,
) -> Result<Value, String> {
    let url = format!(
        "{}/session/{}/{}",
        server_url.trim_end_matches('/'),
        driver.session_id(),
        endpoint
    );
    let mut request = reqwest::Client::builder()
        .timeout(SCRIPT_TIMEOUT)
        .build()
        .map_err(|error| format!("the HTTP client must build: {error}"))?
        .request(method, &url);
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("the WebAuthn command {endpoint} must be sent: {error}"))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| format!("the response of {endpoint} must be read: {error}"))?;
    let value: Value = serde_json::from_str(&text).map_err(|error| {
        format!("the response of {endpoint} must be JSON but was {text}: {error}")
    })?;
    if !status.is_success() {
        return Err(format!(
            "the WebAuthn command {endpoint} failed with {status}: {value}"
        ));
    }
    Ok(value)
}

/// ChromeDriver の WebAuthn の拡張コマンドで仮想認証器を 1 つ付け、その ID を返す。
///
/// 設定は移行前の `support/cdp.rs` の `AddVirtualAuthenticator` (0005) を写す。Backend は
/// `userVerification: required` を要求するため、既定値に依存しない。
pub async fn add_virtual_authenticator(
    driver: &WebDriver,
    server_url: &str,
) -> Result<String, String> {
    let options = json!({
        "protocol": "ctap2",
        "transport": "internal",
        "hasResidentKey": true,
        "hasUserVerification": true,
        "isUserVerified": true,
        "automaticPresenceSimulation": true,
    });
    let body = web_authn_command(
        driver,
        server_url,
        reqwest::Method::POST,
        "webauthn/authenticator",
        Some(options),
    )
    .await?;
    body.get("value")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            format!("the virtual authenticator must be added but the response was {body}")
        })
}

/// ChromeDriver のセッションで、仮想認証器を付けたテストページを操作する。
pub struct TestBrowser {
    /// WebDriver のセッション。Drop で終了するために Option で持つ (`quit` が self を取るため)。
    driver: Option<WebDriver>,
    /// 非同期の WebDriver API を同期のテストから実行するランタイム。
    runtime: tokio::runtime::Runtime,
    /// chromedriver を生かしておくために持つ (最後の 1 つが落ちると停止する)。
    _chromedriver: Arc<ChromeDriver>,
    /// 仮想認証器の拡張コマンドに使う chromedriver の URL。
    server_url: String,
    /// 付けた仮想認証器の ID。
    authenticator_id: String,
}

impl TestBrowser {
    /// Chrome を起動し、`url` を開き、仮想認証器を 1 つ付ける。
    pub fn open(url: &str) -> Result<Self, String> {
        let chromedriver = shared_chromedriver("backend:test-integration")?;
        let server_url = format!("http://127.0.0.1:{}", chromedriver.port());
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|error| format!("the tokio runtime must start: {error}"))?;
        let (driver, authenticator_id) = runtime.block_on(async {
            let mut caps = DesiredCapabilities::chrome();
            for arg in [
                // 新しい headless。ダウンロードと WebAuthn が動く (旧 headless はダウンロード不可)。
                "--headless=new",
                // CI のコンテナでも起動できるように sandbox を切る。
                "--no-sandbox",
                // 共有メモリの小さい環境 (コンテナ) でも落ちないようにする。
                "--disable-dev-shm-usage",
                "--disable-gpu",
            ] {
                caps.add_arg(arg)
                    .map_err(|error| format!("the chrome argument {arg} must be set: {error}"))?;
            }
            let driver = WebDriver::new(&server_url, caps).await.map_err(|error| {
                format!(
                    "Chrome must open a session (run this test with `mise run backend:test-integration`): {error}"
                )
            })?;
            driver
                .set_script_timeout(SCRIPT_TIMEOUT)
                .await
                .map_err(|error| format!("the script timeout must be set: {error}"))?;
            let authenticator_id = add_virtual_authenticator(&driver, &server_url).await?;
            driver
                .goto(url)
                .await
                .map_err(|error| format!("the test page must load: {error}"))?;
            Ok::<_, String>((driver, authenticator_id))
        })?;
        Ok(Self {
            driver: Some(driver),
            runtime,
            _chromedriver: chromedriver,
            server_url,
            authenticator_id,
        })
    }

    /// 使用中の WebDriver のセッション。
    fn driver(&self) -> &WebDriver {
        self.driver
            .as_ref()
            .expect("the session must live while the browser is used")
    }

    /// ページの式を評価し、返った値を JSON にする。式は値でも Promise でもよい。
    /// 式は `window.brewBookTest` の関数を `await` で呼ぶ形にする。
    ///
    /// WebDriver の非同期スクリプトはコールバックで結果を返す。式が例外を投げたときは
    /// `{ ok: false, error }` にして Rust 側で Err に戻す (例外を値にしない)。
    pub fn evaluate_json(&self, expression: &str) -> Result<Value, String> {
        let script = format!(
            "const done = arguments[arguments.length - 1]; \
             (async () => {{ \
               try {{ \
                 const value = await ({expression}); \
                 if (value === undefined) {{ \
                   done({{ ok: false, error: 'the script must return a value' }}); \
                 }} else {{ \
                   done({{ ok: true, value }}); \
                 }} \
               }} catch (error) {{ \
                 done({{ ok: false, error: String(error) }}); \
               }} \
             }})();"
        );
        let ret = self
            .runtime
            .block_on(self.driver().execute_async(&script, Vec::<Value>::new()))
            .map_err(|error| format!("the script must run: {error}"))?;
        let response = ret.json();
        if response.get("ok").and_then(Value::as_bool) != Some(true) {
            let error = response
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("the script must return a value");
            return Err(format!("the script must run: {error}"));
        }
        response
            .get("value")
            .cloned()
            .ok_or_else(|| format!("the script must return a value: {response}"))
    }

    /// 仮想認証器のクレデンシャルの署名カウンタを付け替える。クレデンシャルが無ければ失敗する。
    pub fn set_sign_count(&self, credential_id: &str, sign_count: u32) -> Result<(), String> {
        let wanted = canonical_credential_id(credential_id);
        let list_endpoint = format!(
            "webauthn/authenticator/{}/credentials",
            self.authenticator_id
        );
        let response = self.runtime.block_on(web_authn_command(
            self.driver(),
            &self.server_url,
            reqwest::Method::GET,
            &list_endpoint,
            None,
        ))?;
        let credentials = response
            .get("value")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                format!("the credentials must be listed but the response was {response}")
            })?;
        let credential = credentials
            .iter()
            .find(|credential| {
                canonical_credential_id(
                    credential
                        .get("credentialId")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                ) == wanted
            })
            .ok_or_else(|| {
                format!("the virtual authenticator has no credential {credential_id}")
            })?;
        let stored_id = credential
            .get("credentialId")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("the credential must have an id: {credential}"))?;
        let remove_endpoint = format!(
            "webauthn/authenticator/{}/credentials/{stored_id}",
            self.authenticator_id
        );
        self.runtime.block_on(web_authn_command(
            self.driver(),
            &self.server_url,
            reqwest::Method::DELETE,
            &remove_endpoint,
            None,
        ))?;
        // 入れ直す本体は、取得した項目から既知の項目だけを写す (実装が返す追加の項目を送らない)。
        let mut replacement = json!({
            "credentialId": credential.get("credentialId").cloned().unwrap_or(Value::Null),
            "isResidentCredential": credential
                .get("isResidentCredential")
                .cloned()
                .unwrap_or(Value::Bool(false)),
            "rpId": credential.get("rpId").cloned().unwrap_or(Value::Null),
            "privateKey": credential.get("privateKey").cloned().unwrap_or(Value::Null),
            "signCount": sign_count,
        });
        if let Some(user_handle) = credential.get("userHandle") {
            if !user_handle.is_null() {
                replacement["userHandle"] = user_handle.clone();
            }
        }
        let add_endpoint = format!(
            "webauthn/authenticator/{}/credential",
            self.authenticator_id
        );
        self.runtime.block_on(web_authn_command(
            self.driver(),
            &self.server_url,
            reqwest::Method::POST,
            &add_endpoint,
            Some(replacement),
        ))?;
        Ok(())
    }
}

impl Drop for TestBrowser {
    fn drop(&mut self) {
        if let Some(driver) = self.driver.take() {
            let _ = self.runtime.block_on(driver.quit());
        }
    }
}

/// クレデンシャル ID を同じ形にする。
/// ChromeDriver は base64url (パディングなし) で返し、API も base64url (パディングなし) で返す。
fn canonical_credential_id(id: &str) -> String {
    let normalized = id.replace('+', "-").replace('/', "_");
    let trimmed = normalized.trim_end_matches('=');
    match brew_book_core::base64url::decode(trimmed) {
        Ok(bytes) => brew_book_core::base64url::encode(&bytes),
        Err(_) => id.to_owned(),
    }
}

/// JS の文字列リテラルにする。
pub fn js_string(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}
