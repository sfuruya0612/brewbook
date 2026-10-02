//! 実バックエンドと仮想認証器を使う E2E (0044)。
//!
//! 次の 4 つを組み合わせて、登録、ログイン、ログアウト、抽出の保存、エクスポートの
//! ダウンロード、画面数の成功指標、スクリーンショットの比較を 1 本のテストで実行する。
//!
//! - `wrangler dev`: feature `e2e` を有効にした Dioxus の Web ビルド
//!   (`frontend/target/dx/brew_book_frontend/release/web/public`、`mise run frontend:build-e2e`) を
//!   Static Assets として配信し、`/api/*` を Rust の処理に渡す。実際の画面はこのオリジンから読む。
//! - chromedriver: WebDriver のセッションを開き、`thirtyfour` で Chrome を操作する
//!   (`frontend:test-same-origin`)。仮想認証器は ChromeDriver の WebAuthn の拡張コマンド
//!   (`POST /session/{id}/webauthn/authenticator`) で付ける (ADR-0013 を改訂)。
//! - テスト用の feature `e2e`: ビルドの `data-e2e` の印で、配布用のビルドを配信していないことを
//!   確かめる。テストコードは画面に埋め込まず、ハーネス側で操作する。
//! - 画面数の成功指標: アプリのルート要素の `data-route` 属性を MutationObserver で記録し、
//!   ホームから抽出の保存までの画面の種類を数える (PRD の成功指標)。
//!
//! 利用者、登録用トークン、店、商品、購入、抽出はローカルの D1 に直接投入する (0005 の下ごしらえ)。
//! 写真のアップロードは R2 の資格情報 (`backend/brew_book/.dev.vars`) がある環境でのみ確認し、
//! 無い環境では対象外として記録する (issue 0044)。
//! テスト名の `wrangler_` は、`wrangler dev` を起動するテストを `backend:test` が名前で除外するための規約。

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use support::e2e::{decode_standard_base64, E2eBrowser, E2E_MARKER};
use support::seed::{self, Seed};
use support::{free_port, DevServer};

/// 下ごしらえに使う時刻 (ISO 8601 UTC の固定長)。
const CREATED: &str = "2026-09-01T00:00:00.000Z";
const FUTURE: &str = "2099-01-01T00:00:00.000Z";
/// 下ごしらえした商品の名前。
const PRODUCT_NAME: &str = "E2E Product";
/// 下ごしらえした購入の購入日。
const PURCHASED_ON: &str = "2026-09-21";
/// 登録するパスキーの名前。
const PASSKEY_NAME: &str = "e2e-passkey";
/// 画面のテーマの並び (0039)。
const THEMES: [&str; 2] = ["paper", "night"];
/// ダウンロードを待つ上限。
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60);
/// 失敗したときに報告するログの末尾の行数。
const LOG_TAIL_LINES: usize = 80;

#[test]
fn wrangler_web_routes_registration_login_and_brew_save_ok() {
    let assets = e2e_assets();

    // 下ごしらえ: 利用者、登録用トークン、店、商品 (タグ付き)、購入、抽出。
    // 購入は抽出の保存の画面が選ぶため、抽出は詳細の画面のスクリーンショットのために入れる。
    let mut seed = Seed::new();
    let user = seed::user_id(1);
    seed.user(&user, "E2E の利用者", CREATED);
    let token = seed.registration_token(&user, FUTURE);
    let shop = seed.shop(&user, "E2E の店", None, CREATED, CREATED, None);
    let product = seed.product(&user, PRODUCT_NAME, CREATED, CREATED, None);
    let tag = seed.flavor_tag(&user, "E2E のタグ");
    seed.product_flavor_tag(&user, &product.id, &tag);
    let purchase = seed.purchase(
        &user,
        &product.id,
        Some(&shop.id),
        PURCHASED_ON,
        CREATED,
        CREATED,
        None,
    );
    let brew = seed.brew_with_numbers(
        &user,
        &purchase.id,
        "2026-09-22T08:00:00.000Z",
        Some(15.0),
        Some(250.0),
        Some(92.0),
        Some(150),
        Some(4),
        CREATED,
        CREATED,
        None,
    );

    let server = DevServer::start_with_assets(
        // パスキーの Relying Party ID と Origin を、ブラウザが開くオリジン (`http://localhost:<ポート>`) に
        // 合わせる。`wrangler.toml` の `[vars]` の値は本番 (workers.dev) のため、テストは上書きする。
        |port| {
            vec![
                ("RP_ID".to_owned(), "localhost".to_owned()),
                ("ORIGIN".to_owned(), format!("http://localhost:{port}")),
            ]
        },
        &seed.sql(),
        Some(&assets),
    )
    .expect("wrangler dev must start");

    // ブラウザを起動する chromedriver。空きポートで待ち受ける。
    let driver_port = free_port().expect("a free port for chromedriver must be found");
    let mut chromedriver = start_chromedriver(driver_port);
    if let Err(error) = wait_for_chromedriver(driver_port) {
        let _ = chromedriver.kill();
        let _ = chromedriver.wait();
        panic!("{error}");
    }

    let fixture = Fixture {
        token,
        brew_id: brew.id,
        purchase_id: purchase.id,
        product_id: product.id,
        shop_id: shop.id,
    };
    let work_dir = std::env::temp_dir().join(format!("brewbook-e2e-{}", std::process::id()));
    let result = run_e2e(&server, driver_port, &work_dir, &fixture);

    // 後始末。dev サーバーは DevServer の Drop が止める (workerd の子プロセスも止める)。
    let _ = chromedriver.kill();
    let _ = chromedriver.wait();

    if let Err(error) = result {
        panic!(
            "the same-origin E2E test failed: {error}\nthe wrangler dev output was:\n{}",
            output_tail(&server.output())
        );
    }
}

/// 下ごしらえした行の ID と登録用トークン。
struct Fixture {
    token: String,
    brew_id: String,
    purchase_id: String,
    product_id: String,
    shop_id: String,
}

/// E2E の本体。非同期の WebDriver の操作を 1 つのランタイムで実行する。
fn run_e2e(
    server: &DevServer,
    driver_port: u16,
    work_dir: &Path,
    fixture: &Fixture,
) -> Result<(), String> {
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|error| format!("the tokio runtime must start: {error}"))?;
    runtime.block_on(run_async(server, driver_port, work_dir, fixture))
}

/// E2E の本体 (非同期)。
async fn run_async(
    server: &DevServer,
    driver_port: u16,
    work_dir: &Path,
    fixture: &Fixture,
) -> Result<(), String> {
    let download_dir = work_dir.join("downloads");
    fs::create_dir_all(&download_dir)
        .map_err(|error| format!("failed to create {}: {error}", download_dir.display()))?;
    let report_dir = report_dir();
    fs::create_dir_all(&report_dir)
        .map_err(|error| format!("failed to create {}: {error}", report_dir.display()))?;
    let mut report = Report::new(&report_dir);

    let driver_url = format!("http://127.0.0.1:{driver_port}");
    let browser = E2eBrowser::open(&driver_url, &server.localhost_url(), &download_dir).await?;

    let result = run_steps(&browser, fixture, work_dir, &mut report).await;
    // 失敗の経路でもブラウザを閉じる (0044 のレビューの指摘)。
    let quit = browser.quit().await;
    result.and(quit)
}

/// ブラウザを開いた後の手順 (0044)。失敗しても呼び出し元がブラウザを閉じる。
async fn run_steps(
    browser: &E2eBrowser,
    fixture: &Fixture,
    work_dir: &Path,
    report: &mut Report,
) -> Result<(), String> {
    let download_dir = work_dir.join("downloads");
    let report_dir = report_dir();
    // 仮想認証器を付ける (CTAP2、内部認証器、discoverable なクレデンシャル、利用者の確認に
    // 常に成功する。Backend は `userVerification: required` を要求するため既定値に依存しない)。
    browser.add_virtual_authenticator().await?;

    // 配信されているビルドがテスト用の feature `e2e` を有効にしたものであることを、
    // 実行中の印でも確かめる (静的検査は e2e_assets が行う)。印はアプリの起動時に付くため、
    // 先にオリジンのページを開き、アプリが起動する (data-route が付く) のを待つ。
    browser.goto("/login").await?;
    browser.wait_route("/login").await?;
    let marker = browser
        .eval_string("return document.documentElement.getAttribute('data-e2e');")
        .await?;
    if marker.as_deref() != Some(E2E_MARKER) {
        return Err(format!(
            "the served build must be the E2E build with the {E2E_MARKER} marker but was {marker:?} (run `mise run frontend:build-e2e`)"
        ));
    }

    // サインアウトのうちに認証の画面のスクリーンショットを取る (サインイン中は
    // 遷移の判定がログインと登録をホームへ戻すため。0040)。
    auth_screenshots(browser, report).await?;

    // 登録用トークンでパスキーを登録する (FR-1)。仮想認証器がクレデンシャルを作る。
    browser
        .goto(&format!("/register?token={}", fixture.token))
        .await?;
    browser.wait_route("/register").await?;
    browser.wait_text("Register a passkey").await?;
    browser.type_selector("input", PASSKEY_NAME).await?;
    browser.click_text("Register").await?;
    // 登録が成功するとセッションが発行され、遷移の判定がホームへ戻す。
    browser.wait_route("/").await?;
    browser.wait_text("Log a brew").await?;

    // 画面数の成功指標 (PRD の成功指標)。ここから抽出の保存までを数える。
    browser.start_route_recording().await?;
    save_brew(browser).await?;
    let routes: Vec<String> = browser.recorded_routes().await?;
    let mut kinds: Vec<&String> = routes.iter().collect();
    kinds.sort();
    kinds.dedup();
    if kinds.len() > 3 {
        return Err(format!(
            "the screen count must be 3 or fewer from the home to saving a brew but was {} ({routes:?})",
            kinds.len()
        ));
    }
    for expected in ["/", "/brews/new"] {
        if !kinds.iter().any(|route| route.as_str() == expected) {
            return Err(format!(
                "the screen count must include {expected} but was {routes:?}"
            ));
        }
    }
    println!("screen count: {routes:?} (3 or fewer)");

    // 保存の完了の通知 (Snackbar) を、消える前に Feedback の原本と比較する。
    if let Some(snack) = browser.element_screenshot_base64(".snack").await? {
        capture(
            browser,
            report,
            "Feedback (snackbar)",
            "paper",
            "Feedback",
            &snack,
            "the saved notice on the home after saving a brew",
        )
        .await?;
    }

    // ログアウトしてから、同じパスキーでログインする (FR-2、FR-4)。
    logout(browser).await?;
    login(browser).await?;

    // エクスポートのダウンロード (FR-14、0043)。確認のダイアログ無しで一時ディレクトリへ保存する。
    let export_path = download_dir.join("brewbook-export.json");
    // 前回の実行の残りを拾わないように、先に消す (0044 のレビューの指摘)。
    let _ = fs::remove_file(&export_path);
    browser.goto("/settings").await?;
    browser.wait_route("/settings").await?;
    browser.wait_text("Download the export").await?;
    browser.click_text("Download the export").await?;
    wait_download(&export_path).await?;
    check_export(&export_path)?;
    println!("export: downloaded {}", export_path.display());

    // 主要な画面と部品のスクリーンショットを Paper と Night で取り、原本と比較する (0039)。
    screenshots(browser, report, fixture).await?;
    wide_screenshot(browser, report).await?;

    // 設定の画面のログアウトでも、ログイン画面へ戻ることを確かめる (0043)。
    logout_from_settings(browser).await?;

    // 必須の比較の件数を確かめる (要素が見つからないまま無言で減らないようにする。
    // 消え得るスナックバーだけは件数に含めない。0044 のレビューの指摘)。
    let screens = screens(fixture);
    let required = screens.len() * (THEMES.len() + 1)
        + THEMES.len()
        + COMPONENTS.len() * THEMES.len()
        + THEMES.len()
        + THEMES.len()
        + THEMES.len();
    if report.entries.len() < required {
        return Err(format!(
            "the screenshot comparison must have at least {required} entries but had {}",
            report.entries.len()
        ));
    }

    report.write()?;
    println!(
        "screenshot comparison: {} entries written to {}",
        report.entries.len(),
        report_dir.display()
    );

    Ok(())
}

/// ログインの画面と登録の画面のスクリーンショットを両テーマで取る (0040、0039)。
async fn auth_screenshots(browser: &E2eBrowser, report: &mut Report) -> Result<(), String> {
    for theme in THEMES {
        browser.set_prefers_color_scheme(theme).await?;
        browser.goto("/login").await?;
        browser.wait_route("/login").await?;
        browser.wait_theme(theme).await?;
        browser.wait_fonts().await?;
        let screenshot = browser.screenshot_base64().await?;
        capture(
            browser,
            report,
            "Auth (login)",
            theme,
            "Auth",
            &screenshot,
            "the login screen",
        )
        .await?;
        let button = browser
            .element_screenshot_base64(".btn.primary")
            .await?
            .ok_or_else(|| "the primary button on the login screen must be captured".to_string())?;
        capture(
            browser,
            report,
            "Button (primary)",
            theme,
            "Button",
            &button,
            "the primary button on the login screen",
        )
        .await?;
    }
    Ok(())
}

/// ホームから抽出の保存までを操作する (画面数の成功指標を数える区間)。
async fn save_brew(browser: &E2eBrowser) -> Result<(), String> {
    browser.click_text("Log a brew").await?;
    browser.wait_route("/brews/new").await?;
    // 購入はボトムシートから選ぶ (画面は増えない。PRD の成功指標)。
    browser.click_selector(".picker").await?;
    browser.wait_text(PRODUCT_NAME).await?;
    browser.click_text(PRODUCT_NAME).await?;
    browser.wait_text("Save").await?;
    browser.click_text("Save").await?;
    // 保存の完了は通知で示す (画面には数えない)。
    browser.wait_text("Saved.").await?;
    browser.wait_route("/").await?;
    // 保存した抽出がホームの一覧に出る (抽出の保存まで)。
    browser.wait_text(PRODUCT_NAME).await?;
    Ok(())
}

/// ログアウトし、保護された画面がログインへ戻る (401) ことを確かめる (FR-2、FR-4)。
async fn logout(browser: &E2eBrowser) -> Result<(), String> {
    browser.click_selector("button[aria-label='Menu']").await?;
    browser.wait_text("Log out").await?;
    browser.click_text("Log out").await?;
    browser.wait_route("/login").await?;
    // セッションが失われたことを、保護された画面がログインへ戻ることで確かめる。
    browser.goto("/settings").await?;
    browser.wait_route("/login").await?;
    Ok(())
}

/// 設定の画面からログアウトし、ログインへ戻ることを確かめる (FR-4、0043)。
///
/// ホームのメニューのログアウト (上の [`logout`]) とは別の入口 (設定の画面の文字ボタン) を
/// 確かめる (0044 のレビューの指摘)。
async fn logout_from_settings(browser: &E2eBrowser) -> Result<(), String> {
    browser.goto("/settings").await?;
    browser.wait_route("/settings").await?;
    browser.wait_text("Log out").await?;
    browser.click_text("Log out").await?;
    browser.wait_route("/login").await?;
    Ok(())
}

/// パスキーでログインする (FR-2)。
async fn login(browser: &E2eBrowser) -> Result<(), String> {
    browser.click_text("Log in with a passkey").await?;
    browser.wait_route("/").await?;
    browser.wait_text("Log a brew").await?;
    Ok(())
}

/// 比較の対象にする画面 (スクリーンショットと、Paper と Night の差の検証に使う)。
fn screens(fixture: &Fixture) -> Vec<Screen> {
    vec![
        Screen {
            name: "Home",
            path: "/".to_string(),
            route: "/",
            reference: "Home",
        },
        Screen {
            name: "BrewForm",
            path: "/brews/new".to_string(),
            route: "/brews/new",
            reference: "BrewForm",
        },
        Screen {
            name: "BrewDetail",
            path: format!("/brews/{}", fixture.brew_id),
            route: "/brews/:id",
            reference: "Detail",
        },
        Screen {
            name: "Purchases",
            path: "/purchases".to_string(),
            route: "/purchases",
            reference: "Lists",
        },
        Screen {
            name: "PurchaseDetail",
            path: format!("/purchases/{}", fixture.purchase_id),
            route: "/purchases/:id",
            reference: "Detail",
        },
        Screen {
            name: "Products",
            path: "/products".to_string(),
            route: "/products",
            reference: "Lists",
        },
        Screen {
            name: "ProductEdit",
            path: format!("/products/{}/edit", fixture.product_id),
            route: "/products/:id/edit",
            reference: "RecordForms",
        },
        Screen {
            name: "Shops",
            path: "/shops".to_string(),
            route: "/shops",
            reference: "Lists",
        },
        Screen {
            name: "ShopEdit",
            path: format!("/shops/{}/edit", fixture.shop_id),
            route: "/shops/:id/edit",
            reference: "RecordForms",
        },
        Screen {
            name: "Stats",
            path: "/stats".to_string(),
            route: "/stats",
            reference: "Stats",
        },
        Screen {
            name: "Settings",
            path: "/settings".to_string(),
            route: "/settings",
            reference: "Settings",
        },
    ]
}

/// 主要な画面と部品のスクリーンショットを両テーマで取り、原本と比較する (0039、0044)。
async fn screenshots(
    browser: &E2eBrowser,
    report: &mut Report,
    fixture: &Fixture,
) -> Result<(), String> {
    let screens = screens(fixture);
    for theme in THEMES {
        browser.set_prefers_color_scheme(theme).await?;
        for screen in &screens {
            browser.goto(&screen.path).await?;
            browser.wait_route(screen.route).await?;
            browser.wait_theme(theme).await?;
            browser.wait_fonts().await?;
            let screenshot = browser.screenshot_base64().await?;
            capture(
                browser,
                report,
                screen.name,
                theme,
                screen.reference,
                &screenshot,
                &format!("the screen {}", screen.route),
            )
            .await?;
            // 画面に出ている部品 (0039) も同じテーマで取る。
            for component in COMPONENTS
                .iter()
                .filter(|component| component.screen == screen.name)
            {
                let element = browser
                    .element_screenshot_base64(component.selector)
                    .await?
                    .ok_or_else(|| {
                        format!(
                            "the element {} on the screen {} must be captured (0039)",
                            component.selector, screen.route
                        )
                    })?;
                capture(
                    browser,
                    report,
                    component.name,
                    theme,
                    component.reference,
                    &element,
                    &format!("{} on the screen {}", component.selector, screen.route),
                )
                .await?;
            }
            // フォームの検証のバナー (Feedback) は、購入を選ばずに保存を押すと出る (0041)。
            if screen.name == "BrewForm" {
                browser.click_text("Save").await?;
                browser.wait_text("Check the input.").await?;
                if let Some(banner) = browser.element_screenshot_base64(".banner").await? {
                    capture(
                        browser,
                        report,
                        "Feedback (banner)",
                        theme,
                        "Feedback",
                        &banner,
                        "the validation banner on the brew form",
                    )
                    .await?;
                }
            }
            // 同じ画面の Paper と Night が違うことを確かめる (テーマの切り替え。0039)。
            if theme == "night" {
                let paper = report.dir.join(format!("{}-paper.png", screen.name));
                if paper.is_file() {
                    let diff = browser.compare_images(&screenshot, &paper).await?;
                    if diff.mean <= 10.0 {
                        return Err(format!(
                            "the theme of {} must change the screenshot but the difference was {}",
                            screen.name, diff.mean
                        ));
                    }
                    report.add(json!({
                        "item": screen.name,
                        "theme": "paper vs night",
                        "reference": "the same screen",
                        "mean": diff.mean,
                        "max": diff.max,
                        "differing": diff.differing,
                        "screenshot": [diff.screenshot.0, diff.screenshot.1],
                        "reference_size": [diff.reference.0, diff.reference.1],
                        "note": "the theme change check",
                    }));
                }
            }
        }
    }
    Ok(())
}

/// 2 段組の WideLayout を幅 1280 px で取る (0039)。
async fn wide_screenshot(browser: &E2eBrowser, report: &mut Report) -> Result<(), String> {
    browser.set_window_size(1280, 900).await?;
    for theme in THEMES {
        browser.set_prefers_color_scheme(theme).await?;
        browser.goto("/").await?;
        browser.wait_route("/").await?;
        browser.wait_theme(theme).await?;
        browser.wait_fonts().await?;
        let screenshot = browser.screenshot_base64().await?;
        capture(
            browser,
            report,
            "WideLayout",
            theme,
            "WideLayout",
            &screenshot,
            "the home at 1280 px (the navigation rail and the list)",
        )
        .await?;
    }
    browser.set_window_size(390, 844).await?;
    Ok(())
}

/// 比較の対象にする画面。
struct Screen {
    /// レポートとファイル名に使う名前。
    name: &'static str,
    /// 開くパス。
    path: String,
    /// 待つ `data-route`。
    route: &'static str,
    /// `docs/design/screenshots/` の原本の名前。
    reference: &'static str,
}

/// 比較の対象にする部品 (0039)。
struct Component {
    /// レポートとファイル名に使う名前。
    name: &'static str,
    /// 取る画面の名前。
    screen: &'static str,
    /// 要素の CSS の選択子。
    selector: &'static str,
    /// `docs/design/screenshots/` の原本の名前。
    reference: &'static str,
}

/// 画面に出ている部品 (0039 の 10 種) と、それを取る画面。
const COMPONENTS: &[Component] = &[
    Component {
        name: "AppBar",
        screen: "Home",
        selector: ".appbar",
        reference: "AppBar",
    },
    Component {
        name: "Rating",
        screen: "Home",
        selector: ".rating",
        reference: "Rating",
    },
    Component {
        name: "ListRow",
        screen: "Home",
        selector: ".row",
        reference: "ListRow",
    },
    Component {
        name: "Chip",
        screen: "Products",
        selector: ".chip",
        reference: "Chip",
    },
    Component {
        name: "Ledger",
        screen: "BrewDetail",
        selector: ".ledger",
        reference: "Ledger",
    },
    Component {
        name: "ReferenceTile",
        screen: "BrewDetail",
        selector: ".tile",
        reference: "ReferenceTile",
    },
    Component {
        name: "Field",
        screen: "BrewForm",
        selector: ".field",
        reference: "Field",
    },
    Component {
        name: "Charts",
        screen: "Stats",
        selector: ".chart-frame",
        reference: "Charts",
    },
];

/// 比較の結果の置き場。
struct Report {
    /// 置き場のディレクトリ (スクリーンショットもここに置く)。
    dir: PathBuf,
    /// 比較の 1 件ずつの結果。
    entries: Vec<Value>,
}

impl Report {
    /// 置き場を作る。
    fn new(dir: &Path) -> Self {
        Self {
            dir: dir.to_path_buf(),
            entries: Vec::new(),
        }
    }

    /// 1 件の結果を加え、テストの出力にも 1 行で出す。
    fn add(&mut self, entry: Value) {
        println!(
            "screenshot comparison: {}",
            serde_json::to_string(&entry).unwrap_or_default()
        );
        self.entries.push(entry);
    }

    /// 結果を JSON に書く。
    fn write(&self) -> Result<(), String> {
        let path = self.dir.join("comparison.json");
        let text = serde_json::to_string_pretty(&self.entries)
            .map_err(|error| format!("the comparison must be JSON: {error}"))?;
        fs::write(&path, text)
            .map_err(|error| format!("failed to write {}: {error}", path.display()))
    }
}

/// スクリーンショットを保存し、原本と比較して結果を記録する。
#[allow(clippy::too_many_arguments)]
async fn capture(
    browser: &E2eBrowser,
    report: &mut Report,
    item: &str,
    theme: &str,
    reference_name: &str,
    screenshot_base64: &str,
    note: &str,
) -> Result<(), String> {
    let screenshot_path = report.dir.join(format!("{item}-{theme}.png"));
    let bytes = decode_standard_base64(screenshot_base64)?;
    fs::write(&screenshot_path, &bytes)
        .map_err(|error| format!("failed to write {}: {error}", screenshot_path.display()))?;
    let reference_path = reference_path(reference_name, theme);
    let diff = browser
        .compare_images(screenshot_base64, &reference_path)
        .await?;
    report.add(json!({
        "item": item,
        "theme": theme,
        "reference": format!("docs/design/screenshots/{reference_name}-{theme}.png"),
        "mean": diff.mean,
        "max": diff.max,
        "differing": diff.differing,
        "screenshot": [diff.screenshot.0, diff.screenshot.1],
        "reference_size": [diff.reference.0, diff.reference.1],
        "note": note,
    }));
    Ok(())
}

/// 比較の結果とスクリーンショットの置き場 (`frontend/target/e2e-screenshots/`、git 管理外)。
fn report_dir() -> PathBuf {
    frontend_dir().join("target/e2e-screenshots")
}

/// `docs/design/screenshots/` の原本のパス。
fn reference_path(name: &str, theme: &str) -> PathBuf {
    repo_root()
        .join("docs/design/screenshots")
        .join(format!("{name}-{theme}.png"))
}

/// リポジトリのルート。
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// リポジトリの `frontend` ディレクトリ。
fn frontend_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../frontend")
}

/// Dioxus の E2E ビルドを検査して、配信するディレクトリを返す。
///
/// ビルドの成果物と、feature `e2e` の印 (wasm のマーカー) を確かめる。配布用のビルドを
/// 配信してしまうと、テストの途中まで気付けないためである。
fn e2e_assets() -> PathBuf {
    let public = frontend_dir().join("target/dx/brew_book_frontend/release/web/public");
    let index = public.join("index.html");
    assert!(
        index.is_file(),
        "the E2E web build must exist at {} (run `mise run frontend:build-e2e`)",
        public.display()
    );
    for name in ["tailwind.css", "favicon.png", "manifest.json"] {
        assert!(
            public.join(name).is_file(),
            "{} must exist (run `mise run frontend:build-e2e`)",
            public.join(name).display()
        );
    }
    let wasm = find_assets(&public, "wasm");
    assert!(
        !wasm.is_empty(),
        "the E2E build must have an assets/*.wasm (run `mise run frontend:build-e2e`)"
    );
    for path in &wasm {
        let bytes = fs::read(path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        assert!(
            bytes
                .windows(E2E_MARKER.len())
                .any(|window| window == E2E_MARKER.as_bytes()),
            "{} must contain the E2E marker (run `mise run frontend:build-e2e`)",
            path.display()
        );
    }
    let js = find_assets(&public, "js");
    assert!(
        !js.is_empty(),
        "the E2E build must have an assets/*.js (the loader. run `mise run frontend:build-e2e`)"
    );
    public
}

/// `assets/` の中から指定の拡張子のファイルを探す。
fn find_assets(public: &Path, extension: &str) -> Vec<PathBuf> {
    let assets = public.join("assets");
    let mut found: Vec<PathBuf> = fs::read_dir(&assets)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", assets.display()))
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == extension).then_some(path)
        })
        .collect();
    found.sort();
    found
}

/// エクスポートのダウンロードを待つ。
async fn wait_download(path: &Path) -> Result<(), String> {
    let deadline = Instant::now() + DOWNLOAD_TIMEOUT;
    loop {
        if path.is_file() {
            return Ok(());
        }
        if Instant::now() > deadline {
            return Err(format!(
                "the export must be downloaded to {} (0043)",
                path.display()
            ));
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// ダウンロードしたエクスポートが、下ごしらえと保存した記録を持つことを確かめる (FR-14)。
fn check_export(path: &Path) -> Result<(), String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| format!("{} must be JSON: {error}", path.display()))?;
    let products = value
        .get("products")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("the export must have the products array: {value}"))?;
    if !products
        .iter()
        .any(|product| product.get("name").and_then(Value::as_str) == Some(PRODUCT_NAME))
    {
        return Err(format!(
            "the export must contain the product {PRODUCT_NAME}: {value}"
        ));
    }
    let brews = value
        .get("brews")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("the export must have the brews array: {value}"))?;
    // 下ごしらえの 1 件と、E2E で保存した 1 件が入る (0044 のレビューの指摘)。
    if brews.len() < 2 {
        return Err(format!(
            "the export must contain the prepared and the saved brews but had {}: {value}",
            brews.len()
        ));
    }
    Ok(())
}

/// 出力の末尾を読む (失敗の報告に使う)。Worker はリクエスト 1 件ごとに経路名と状態コードを出す。
fn output_tail(lines: &[String]) -> String {
    let start = lines.len().saturating_sub(LOG_TAIL_LINES);
    lines[start..].join("\n")
}

/// chromedriver を起動する。見つからなければ、mise のタスクから実行するよう促す。
fn start_chromedriver(port: u16) -> Child {
    Command::new("chromedriver")
        .arg(format!("--port={port}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| {
            panic!("chromedriver must start (run this test with `mise run frontend:test-same-origin`): {error}")
        })
}

/// chromedriver が待ち受けを始めるまで待つ。
///
/// 起動の直後は接続できないため、WebDriver のセッションを開く前に `/status` を確認する。
fn wait_for_chromedriver(port: u16) -> Result<(), String> {
    let url = format!("http://127.0.0.1:{port}/status");
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Ok(response) = reqwest::blocking::get(&url) {
            if response.status().is_success() {
                return Ok(());
            }
        }
        if Instant::now() > deadline {
            return Err(format!("chromedriver must listen on port {port}"));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
