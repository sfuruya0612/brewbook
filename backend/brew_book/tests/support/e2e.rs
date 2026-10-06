//! WebDriver (chromedriver) で Dioxus の画面を操作する E2E のハーネス (0044)。
//!
//! `frontend:test-same-origin` が `wrangler dev` と chromedriver を起動し、feature `e2e` を
//! 有効にした Dioxus の Web ビルドを Static Assets として配信して、このハーネスが実際の画面を
//! Chrome で操作する。仮想認証器は ChromeDriver の WebAuthn の拡張コマンド
//! (`POST /session/{id}/webauthn/authenticator`) で付ける (ADR-0013 を改訂)。
//!
//! WebDriver のクライアントは thirtyfour を使う。fantoccini と比較し、CDP (テーマの切り替えの
//! `Emulation.setEmulatedMedia` に使う) と、要素と画面のスクリーンショットの API を持つことを
//! 決め手にした (比較は issue 0044 に記録)。WebAuthn の拡張コマンドは thirtyfour に型が無い
//! ため、同じセッションへ reqwest で直接送る。
//!
//! スクリーンショットと `docs/design/screenshots/` の比較は、ブラウザの canvas で 64x64 に
//! 縮小した画像の画素の差を計算する。画像を復号するクレートを増やさず、テーマの色や構図の
//! 大きな食い違いを数値で残すためである。結果の解釈 (許容する差分と直す差分) は issue に記録する。

use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use thirtyfour::prelude::*;

/// 1 つの操作 (経路の変更、要素の表示、文言の表示) を待つ上限。
const STEP_TIMEOUT: Duration = Duration::from_secs(60);
/// 待ちの確認の間隔。
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// E2E のビルドのマーカー。`frontend/src/lib.rs` の `E2E_MARKER` と同じ値にする。
pub const E2E_MARKER: &str = "brewbook-e2e-build";

/// 画面のスクリーンショットの比較の結果。
#[derive(Debug, Clone)]
pub struct ImageDiff {
    /// 縮小した画像の画素の差の平均 (0-255)。
    pub mean: f64,
    /// 画素の差の最大。
    pub max: f64,
    /// 差が閾値を超えた画素の割合 (0-1)。
    pub differing: f64,
    /// スクリーンショットの大きさ (幅、高さ)。
    pub screenshot: (u32, u32),
    /// 原本の大きさ (幅、高さ)。
    pub reference: (u32, u32),
}

/// Chrome と WebDriver のセッション。
pub struct E2eBrowser {
    driver: WebDriver,
    /// chromedriver の URL (WebAuthn の拡張コマンドに使う)。
    server_url: String,
    /// 画面のオリジン (末尾のスラッシュ無し)。
    base_url: String,
}

impl E2eBrowser {
    /// chromedriver に接続し、携帯の幅 (390x844) の headless の Chrome を開く。
    pub async fn open(
        server_url: &str,
        base_url: &str,
        download_dir: &Path,
    ) -> Result<Self, String> {
        let mut caps = DesiredCapabilities::chrome();
        for arg in [
            // 新しい headless。ダウンロードと WebAuthn が動く (旧 headless はダウンロード不可)。
            "--headless=new",
            // CI のコンテナでも起動できるように sandbox を切る。
            "--no-sandbox",
            // 共有メモリの小さい環境 (コンテナ) でも落ちないようにする。
            "--disable-dev-shm-usage",
            "--disable-gpu",
            // 表示の言語を英語に固定して、文言で操作できるようにする (FR-16)。
            "--lang=en-US",
            // 携帯の幅 (390x844)。ホームから抽出の保存までの画面数の成功指標を 1 列の配置で
            // 数えるためである (デザインの WideLayout は 840 px 以上で 2 段組にする)。
            "--window-size=390,844",
        ] {
            caps.add_arg(arg)
                .map_err(|error| format!("the chrome argument {arg} must be set: {error}"))?;
        }
        caps.add_experimental_option(
            "prefs",
            json!({
                "intl.accept_languages": "en-US",
                // エクスポートのダウンロードを、確認のダイアログ無しで一時ディレクトリへ保存する (FR-14)。
                "download.default_directory": download_dir.display().to_string(),
                "download.prompt_for_download": false,
                "download.directory_upgrade": true,
            }),
        )
        .map_err(|error| format!("the chrome preferences must be set: {error}"))?;

        let driver = WebDriver::new(server_url, caps)
            .await
            .map_err(|error| format!("chromedriver must open a session (run this test with `mise run frontend:test-same-origin`): {error}"))?;
        driver
            .set_window_rect(0, 0, 390, 844)
            .await
            .map_err(|error| format!("the window must be sized: {error}"))?;
        Ok(Self {
            driver,
            server_url: server_url.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

    /// セッションを終了する。
    pub async fn quit(self) -> Result<(), String> {
        self.driver
            .quit()
            .await
            .map_err(|error| format!("the session must close: {error}"))
    }

    /// ChromeDriver の WebAuthn の拡張コマンドで仮想認証器を 1 つ付ける (ADR-0004、0044)。
    ///
    /// 設定は移行前の Flutter 版の Chrome DevTools Protocol のもの (0005、
    /// `frontend/test_driver/same_origin_test.dart`。0045 で削除) を写す。Backend は
    /// `userVerification: required` を要求するため、既定値に依存しない。
    pub async fn add_virtual_authenticator(&self) -> Result<(), String> {
        super::browser::add_virtual_authenticator(&self.driver, &self.server_url)
            .await
            .map(|_| ())
    }

    /// ページの式を評価し、文字列の結果を返す (文字列でなければ None)。
    pub async fn eval_string(&self, script: &str) -> Result<Option<String>, String> {
        let ret = self
            .driver
            .execute(script, Vec::<Value>::new())
            .await
            .map_err(|error| format!("the script must run: {error}"))?;
        Ok(ret.json().as_str().map(str::to_string))
    }

    /// ページの式を評価し、JSON の文字列の結果を値にして返す。
    pub async fn eval_json(&self, script: &str) -> Result<Value, String> {
        let text = self
            .eval_string(script)
            .await?
            .ok_or_else(|| "the script must return a JSON string".to_string())?;
        serde_json::from_str(&text)
            .map_err(|error| format!("the result must be JSON but was {text}: {error}"))
    }

    /// パスを開く。
    pub async fn goto(&self, path: &str) -> Result<(), String> {
        self.driver
            .goto(format!("{}{}", self.base_url, path))
            .await
            .map_err(|error| format!("{path} must open: {error}"))
    }

    /// 現在のページを読み直す。
    pub async fn refresh(&self) -> Result<(), String> {
        self.driver
            .refresh()
            .await
            .map_err(|error| format!("the page must reload: {error}"))
    }

    /// 現在の URL のパス。
    pub async fn current_path(&self) -> Result<String, String> {
        let url = self
            .driver
            .current_url()
            .await
            .map_err(|error| format!("the current URL must be read: {error}"))?;
        Ok(url.path().to_string())
    }

    /// `data-route` 属性の値 (アプリのルート要素。まだ無ければ None)。
    pub async fn route(&self) -> Result<Option<String>, String> {
        let ret = self
            .driver
            .execute(
                "const element = document.getElementById('main'); \
                 return element ? element.getAttribute('data-route') : null;",
                Vec::<Value>::new(),
            )
            .await
            .map_err(|error| format!("the data-route must be read: {error}"))?;
        Ok(ret.json().as_str().map(str::to_string))
    }

    /// `data-route` が期待の値になるまで待つ。
    pub async fn wait_route(&self, expected: &str) -> Result<(), String> {
        let deadline = self.deadline();
        loop {
            match self.route().await? {
                Some(route) if route == expected => return Ok(()),
                Some(route) => {
                    if Instant::now() > deadline {
                        return Err(format!(
                            "the route must be {expected} but stayed at {route} (url: {})",
                            self.current_path().await.unwrap_or_default()
                        ));
                    }
                }
                None => {
                    if Instant::now() > deadline {
                        return Err("the app root must have a data-route attribute".to_string());
                    }
                }
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    /// 直接の文字 (子の要素の文字を含まない) が `text` と一致する要素が現れるまで待つ。
    pub async fn wait_text(&self, text: &str) -> Result<(), String> {
        let xpath = format!("//*[normalize-space(text()) = {}]", xpath_literal(text));
        self.wait_element(By::XPath(xpath), text).await.map(|_| ())
    }

    /// CSS の選択子に一致する要素が現れるまで待つ。
    pub async fn wait_selector(&self, selector: &str) -> Result<(), String> {
        self.wait_element(By::Css(selector.to_string()), selector)
            .await
            .map(|_| ())
    }

    /// 要素が現れるまで待って返す。
    pub async fn find(&self, by: By) -> Result<WebElement, String> {
        let label = format!("{by:?}");
        self.wait_element(by, &label).await
    }

    async fn wait_element(&self, by: By, label: &str) -> Result<WebElement, String> {
        let deadline = self.deadline();
        loop {
            match self.driver.find(by.clone()).await {
                Ok(element) => return Ok(element),
                Err(error) => {
                    if Instant::now() > deadline {
                        return Err(format!(
                            "the element {label} must appear (url: {}): {error}",
                            self.current_path().await.unwrap_or_default()
                        ));
                    }
                }
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    /// 直接の文字 (子の要素の文字を含まない) が `text` と一致する要素を押す。
    pub async fn click_text(&self, text: &str) -> Result<(), String> {
        let xpath = format!("//*[normalize-space(text()) = {}]", xpath_literal(text));
        let element = self.find(By::XPath(xpath)).await?;
        element
            .click()
            .await
            .map_err(|error| format!("{text} must be clicked: {error}"))
    }

    /// CSS の選択子に一致する要素を押す。
    pub async fn click_selector(&self, selector: &str) -> Result<(), String> {
        let element = self.find(By::Css(selector.to_string())).await?;
        element
            .click()
            .await
            .map_err(|error| format!("{selector} must be clicked: {error}"))
    }

    /// ハンバーガーメニューの項目 (`.menu` の中の直接の文字) を押す (0047)。
    ///
    /// ナビゲーションレールの項目も同じ文言を持つため、`.menu` の中に限って探す。
    pub async fn click_menu_item(&self, text: &str) -> Result<(), String> {
        let xpath = format!(
            "//div[contains(concat(' ', normalize-space(@class), ' '), ' menu ')]\
             //div[normalize-space(text()) = {}]",
            xpath_literal(text)
        );
        let element = self.find(By::XPath(xpath)).await?;
        element
            .click()
            .await
            .map_err(|error| format!("the menu item {text} must be clicked: {error}"))
    }

    /// ハンバーガーメニューの面 (`.menu`) が閉じるまで待つ (0047)。
    pub async fn wait_menu_closed(&self) -> Result<(), String> {
        let deadline = self.deadline();
        loop {
            if self
                .driver
                .find(By::Css(".menu".to_string()))
                .await
                .is_err()
            {
                return Ok(());
            }
            if Instant::now() > deadline {
                return Err(format!(
                    "the hamburger menu must close (url: {})",
                    self.current_path().await.unwrap_or_default()
                ));
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    /// CSS の選択子に一致する入力欄に文字を入れる。
    pub async fn type_selector(&self, selector: &str, text: &str) -> Result<(), String> {
        let element = self.find(By::Css(selector.to_string())).await?;
        element
            .send_keys(text)
            .await
            .map_err(|error| format!("{selector} must receive the text: {error}"))
    }

    /// CSS の選択子に一致するファイルの入力欄に、パスのファイルを選ばせる。
    pub async fn choose_file_selector(&self, selector: &str, path: &Path) -> Result<(), String> {
        let element = self.find(By::Css(selector.to_string())).await?;
        element
            .send_keys(path.display().to_string())
            .await
            .map_err(|error| {
                format!(
                    "{selector} must receive the file {}: {error}",
                    path.display()
                )
            })
    }

    /// 文字を消してから入れる (前の値が残らないようにする)。
    pub async fn clear_and_type_selector(&self, selector: &str, text: &str) -> Result<(), String> {
        let element = self.find(By::Css(selector.to_string())).await?;
        element
            .clear()
            .await
            .map_err(|error| format!("{selector} must be cleared: {error}"))?;
        element
            .send_keys(text)
            .await
            .map_err(|error| format!("{selector} must receive the text: {error}"))
    }

    /// ブラウザの `prefers-color-scheme` を写して、アプリのテーマを切り替える (0039)。
    ///
    /// アプリの起動スクリプトが `matchMedia` の結果から `data-theme` を決めるため、CDP の
    /// エミュレーションで OS の設定を写す。
    pub async fn set_prefers_color_scheme(&self, theme: &str) -> Result<(), String> {
        let value = if theme == "night" { "dark" } else { "light" };
        self.driver
            .cdp()
            .send_raw(
                "Emulation.setEmulatedMedia",
                json!({ "features": [{ "name": "prefers-color-scheme", "value": value }] }),
            )
            .await
            .map_err(|error| format!("the color scheme must be emulated: {error}"))?;
        Ok(())
    }

    /// `<html>` の `data-theme` の値。
    pub async fn theme(&self) -> Result<Option<String>, String> {
        let ret = self
            .driver
            .execute(
                "return document.documentElement.getAttribute('data-theme');",
                Vec::<Value>::new(),
            )
            .await
            .map_err(|error| format!("the data-theme must be read: {error}"))?;
        Ok(ret.json().as_str().map(str::to_string))
    }

    /// `data-theme` が期待の値になるまで待つ。
    pub async fn wait_theme(&self, expected: &str) -> Result<(), String> {
        let deadline = self.deadline();
        loop {
            match self.theme().await? {
                Some(theme) if theme == expected => return Ok(()),
                Some(theme) => {
                    if Instant::now() > deadline {
                        return Err(format!("the theme must be {expected} but was {theme}"));
                    }
                }
                None => {
                    if Instant::now() > deadline {
                        return Err("the html element must have a data-theme attribute".to_string());
                    }
                }
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }

    /// 書体とアイコンの読み込みが終わるまで待つ。
    pub async fn wait_fonts(&self) -> Result<(), String> {
        let ret = self
            .driver
            .execute_async(
                "const done = arguments[arguments.length - 1]; \
                 document.fonts.ready.then(() => done(true)).catch(() => done(false));",
                Vec::<Value>::new(),
            )
            .await
            .map_err(|error| format!("document.fonts.ready must resolve: {error}"))?;
        match ret.json() {
            Value::Bool(true) => Ok(()),
            _ => Err("the fonts must load".to_string()),
        }
    }

    /// 画面全体のスクリーンショットを PNG の base64 で返す。
    pub async fn screenshot_base64(&self) -> Result<String, String> {
        self.driver
            .screenshot_as_png_base64()
            .await
            .map_err(|error| format!("the screenshot must be taken: {error}"))
    }

    /// CSS の選択子に一致する要素のスクリーンショットを PNG の base64 で返す。
    /// 要素が無いときは None。
    pub async fn element_screenshot_base64(
        &self,
        selector: &str,
    ) -> Result<Option<String>, String> {
        match self.driver.find(By::Css(selector.to_string())).await {
            Ok(element) => element
                .screenshot_as_png_base64()
                .await
                .map(Some)
                .map_err(|error| format!("the screenshot of {selector} must be taken: {error}")),
            Err(_) => Ok(None),
        }
    }

    /// 画面の大きさを変える (2 段組の WideLayout の撮影に使う)。
    pub async fn set_window_size(&self, width: u32, height: u32) -> Result<(), String> {
        self.driver
            .set_window_rect(0, 0, width, height)
            .await
            .map_err(|error| format!("the window must be resized: {error}"))
    }

    /// 表示の領域の大きさを CDP の端末の計測の上書きで変える (0048)。
    ///
    /// OS の窓には下限の幅があり、`set_window_size` では 375 px にできない環境がある
    /// (macOS の headless の Chrome は 500 px に丸める)。症状を計測した幅で確かめるため、
    /// 表示の領域を直接上書きする。`mobile` を false にすると `<meta name="viewport">` を
    /// 効かせず、指定した幅をそのまま使う。
    pub async fn set_viewport_size(&self, width: u32, height: u32) -> Result<(), String> {
        self.driver
            .cdp()
            .send_raw(
                "Emulation.setDeviceMetricsOverride",
                json!({ "width": width, "height": height, "deviceScaleFactor": 1, "mobile": false }),
            )
            .await
            .map_err(|error| format!("the viewport must be overridden: {error}"))?;
        Ok(())
    }

    /// 表示の領域の上書きを外す (0048)。
    pub async fn clear_viewport_size(&self) -> Result<(), String> {
        self.driver
            .cdp()
            .send_raw("Emulation.clearDeviceMetricsOverride", json!({}))
            .await
            .map_err(|error| format!("the viewport override must be cleared: {error}"))?;
        Ok(())
    }

    /// 経路の変更の記録を始める (画面数の成功指標。PRD の成功指標)。
    ///
    /// `data-route` の変更を MutationObserver で記録し、E2E が表示した画面の種類を数えられる
    /// ようにする。ダイアログ、ボトムシート、保存完了の通知は経路ではないため数えない。
    pub async fn start_route_recording(&self) -> Result<(), String> {
        self.driver
            .execute(
                "window.__brewbookRoutes = []; \
                 const element = document.getElementById('main'); \
                 const record = () => { \
                   const route = element.getAttribute('data-route'); \
                   const routes = window.__brewbookRoutes; \
                   if (route && (routes.length === 0 || routes[routes.length - 1] !== route)) { routes.push(route); } \
                 }; \
                 if (window.__brewbookRouteObserver) { window.__brewbookRouteObserver.disconnect(); } \
                 window.__brewbookRouteObserver = new MutationObserver(record); \
                 window.__brewbookRouteObserver.observe(element, { attributes: true, attributeFilter: ['data-route'] }); \
                 record();",
                Vec::<Value>::new(),
            )
            .await
            .map_err(|error| format!("the route recording must start: {error}"))?;
        Ok(())
    }

    /// 記録した経路の並び (表示した順。連続する同じ値は 1 つにする)。
    pub async fn recorded_routes(&self) -> Result<Vec<String>, String> {
        let ret = self
            .driver
            .execute(
                "return JSON.stringify(window.__brewbookRoutes || []);",
                Vec::<Value>::new(),
            )
            .await
            .map_err(|error| format!("the recorded routes must be read: {error}"))?;
        let text = ret
            .json()
            .as_str()
            .ok_or_else(|| format!("the recorded routes must be JSON: {}", ret.json()))?;
        serde_json::from_str(text).map_err(|error| {
            format!("the recorded routes must be an array but were {text}: {error}")
        })
    }

    /// スクリーンショットと原本 (docs/design/screenshots) を比較する。
    pub async fn compare_images(
        &self,
        screenshot_base64: &str,
        reference_path: &Path,
    ) -> Result<ImageDiff, String> {
        let reference_bytes = std::fs::read(reference_path).map_err(|error| {
            format!(
                "the reference screenshot {} must be readable: {error}",
                reference_path.display()
            )
        })?;
        let screenshot = format!("data:image/png;base64,{screenshot_base64}");
        let reference = format!(
            "data:image/png;base64,{}",
            standard_base64(&reference_bytes)
        );
        let ret = self
            .driver
            .execute_async(
                IMAGE_DIFF_SCRIPT,
                vec![Value::String(screenshot), Value::String(reference)],
            )
            .await
            .map_err(|error| format!("the images must be compared: {error}"))?;
        let text = ret
            .json()
            .as_str()
            .ok_or_else(|| format!("the comparison must return JSON: {}", ret.json()))?;
        let value: Value = serde_json::from_str(text)
            .map_err(|error| format!("the comparison must be JSON but was {text}: {error}"))?;
        if let Some(error) = value.get("error") {
            return Err(format!(
                "the images must be compared ({} and {}): {error}",
                reference_path.display(),
                screenshot_base64.len()
            ));
        }
        let number = |key: &str| -> Result<f64, String> {
            value
                .get(key)
                .and_then(Value::as_f64)
                .ok_or_else(|| format!("the comparison must have {key}: {value}"))
        };
        let pair = |key: &str| -> Result<(u32, u32), String> {
            let array = value
                .get(key)
                .and_then(Value::as_array)
                .ok_or_else(|| format!("the comparison must have {key}: {value}"))?;
            Ok((
                array.first().and_then(Value::as_u64).unwrap_or(0) as u32,
                array.get(1).and_then(Value::as_u64).unwrap_or(0) as u32,
            ))
        };
        Ok(ImageDiff {
            mean: number("mean")?,
            max: number("max")?,
            differing: number("differing")?,
            screenshot: pair("screenshot")?,
            reference: pair("reference")?,
        })
    }

    /// 待ちの期限。
    fn deadline(&self) -> Instant {
        Instant::now() + STEP_TIMEOUT
    }
}

/// 比較の JavaScript (実行はブラウザの canvas。画像を復号するクレートを増やさない)。
///
/// 2 つの画像を 64x64 に縮小して描き、画素の差の平均、最大、閾値を超えた画素の割合を返す。
const IMAGE_DIFF_SCRIPT: &str = r#"
const [shot, reference, done] = [arguments[0], arguments[1], arguments[arguments.length - 1]];
const load = (src) => new Promise((resolve, reject) => {
  const image = new Image();
  image.onload = () => resolve(image);
  image.onerror = () => reject(new Error("the image could not be decoded"));
  image.src = src;
});
Promise.all([load(shot), load(reference)]).then(([a, b]) => {
  const size = 64;
  const canvasA = document.createElement("canvas");
  canvasA.width = size;
  canvasA.height = size;
  const canvasB = document.createElement("canvas");
  canvasB.width = size;
  canvasB.height = size;
  const ctxA = canvasA.getContext("2d");
  const ctxB = canvasB.getContext("2d");
  ctxA.drawImage(a, 0, 0, size, size);
  ctxB.drawImage(b, 0, 0, size, size);
  const dataA = ctxA.getImageData(0, 0, size, size).data;
  const dataB = ctxB.getImageData(0, 0, size, size).data;
  let sum = 0;
  let max = 0;
  let differing = 0;
  for (let i = 0; i < dataA.length; i += 4) {
    const d = (Math.abs(dataA[i] - dataB[i]) + Math.abs(dataA[i + 1] - dataB[i + 1]) + Math.abs(dataA[i + 2] - dataB[i + 2])) / 3;
    sum += d;
    if (d > max) { max = d; }
    if (d > 24) { differing += 1; }
  }
  const pixels = dataA.length / 4;
  done(JSON.stringify({
    mean: Math.round((sum / pixels) * 10) / 10,
    max: Math.round(max),
    differing: Math.round((differing / pixels) * 1000) / 1000,
    screenshot: [a.width, a.height],
    reference: [b.width, b.height],
  }));
}).catch((error) => done(JSON.stringify({ error: String(error) })));
"#;

/// 標準の base64 (パディングあり) を base64url に直して復号する。
pub fn decode_standard_base64(text: &str) -> Result<Vec<u8>, String> {
    let url: String = text
        .chars()
        .filter(|character| *character != '=')
        .map(|character| match character {
            '+' => '-',
            '/' => '_',
            other => other,
        })
        .collect();
    brew_book_core::base64url::decode(&url).map_err(|error| error.to_string())
}

/// base64url の符号化を標準の base64 (パディングあり) に直す。
fn standard_base64(bytes: &[u8]) -> String {
    let mut standard: String = brew_book_core::base64url::encode(bytes)
        .chars()
        .map(|character| match character {
            '-' => '+',
            '_' => '/',
            other => other,
        })
        .collect();
    while !standard.len().is_multiple_of(4) {
        standard.push('=');
    }
    standard
}

/// XPath の文字列リテラルにする (文言は ASCII のため引用符だけを扱う)。
fn xpath_literal(text: &str) -> String {
    if text.contains('\'') {
        format!("\"{text}\"")
    } else {
        format!("'{text}'")
    }
}
