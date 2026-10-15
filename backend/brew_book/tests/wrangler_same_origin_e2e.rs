//! 実バックエンドと仮想認証器を使う E2E (0044)。
//!
//! 次の 4 つを組み合わせて、登録、ログイン、ログアウト、抽出の保存、エクスポートの
//! ダウンロード、ハンバーガーメニューの遷移 (0047)、画面数の成功指標、スクリーンショットの
//! 比較を 1 本のテストで実行する。
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
//! 利用者、登録用トークン、店、購入 (タグ付き)、抽出はローカルの D1 に直接投入する (0005 の下ごしらえ)。
//! 写真のアップロードは R2 の資格情報 (`backend/brew_book/.dev.vars`) がある環境でのみ確認し、
//! 無い環境では対象外として記録する (issue 0044)。
//! テスト名の `wrangler_` は、`wrangler dev` を起動するテストを `backend:test` が名前で除外するための規約。

mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use support::e2e::{decode_standard_base64, E2eBrowser, E2E_MARKER};
use support::seed::{self, Seed};
use support::DevServer;

/// 下ごしらえに使う時刻 (ISO 8601 UTC の固定長)。
const CREATED: &str = "2026-09-01T00:00:00.000Z";
const FUTURE: &str = "2099-01-01T00:00:00.000Z";
/// 下ごしらえした購入の商品名 (0057)。
const PURCHASE_NAME: &str = "E2E Beans";
/// 下ごしらえした購入の焙煎度 (過去の購入をもとにした登録の確認に使う。issue 0058)。
const PURCHASE_ROAST: &str = "E2E Medium";
/// 下ごしらえした店の名前。
const SHOP_NAME: &str = "E2E の店";
/// 推測の差し替えが返す商品名と生産者とタグ (FR-19、0057)。
const SUGGESTED_NAME: &str = "E2E Suggested Beans";
const SUGGESTED_PRODUCER: &str = "E2E Suggested Producer";
const SUGGESTED_TAG: &str = "E2E Suggested Tag";
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

    // 下ごしらえ: 利用者、登録用トークン、店、購入 (タグ付き)、抽出。
    // 購入は抽出の保存の画面が選ぶため、抽出は詳細の画面のスクリーンショットのために入れる。
    let mut seed = Seed::new();
    let user = seed::user_id(1);
    seed.user(&user, "E2E の利用者", CREATED);
    let token = seed.registration_token(&user, FUTURE);
    let shop = seed.shop(&user, SHOP_NAME, None, CREATED, CREATED);
    let purchase = seed.purchase_with_roast(
        &user,
        PURCHASE_NAME,
        Some(&shop.id),
        PURCHASED_ON,
        PURCHASE_ROAST,
        CREATED,
        CREATED,
    );
    let tag = seed.flavor_tag(&user, "E2E のタグ");
    seed.purchase_flavor_tag(&user, &purchase.id, &tag);
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
    let chromedriver = support::browser::start_chromedriver("frontend:test-same-origin")
        .expect("chromedriver must start");
    let driver_port = chromedriver.port();

    let fixture = Fixture {
        token,
        brew_id: brew.id,
        purchase_id: purchase.id,
        shop_id: shop.id,
    };
    let work_dir = std::env::temp_dir().join(format!("brewbook-e2e-{}", std::process::id()));
    let result = run_e2e(&server, driver_port, &work_dir, &fixture);

    // 後始末。dev サーバーは DevServer の Drop が止める (workerd の子プロセスも止める)。
    // chromedriver は ChromeDriver の Drop がプロセスグループごと止める (0019)。
    drop(chromedriver);

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

    // ハンバーガーメニューから 5 つの行き先へ移れる (0047、0057)。
    nav_menu(browser).await?;
    println!("nav menu: the five destinations are reachable");

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

    // 幅 375 px で各フォームが横にはみ出さず、縦スクロールで最後の入力欄に届くことを
    // 確かめる (0048)。
    form_layouts(browser, fixture).await?;
    println!("form layouts: the forms fit 375 px and scroll to the last input");

    // 購入の登録で写真を選ぶと、推測した商品の項目がフォームに入ることを確かめる (FR-19、0057)。
    suggestion_fills_the_form(browser).await?;
    println!("suggestion: the suggested name, producer, and tag are filled in the purchase form");

    // 幅 390 px では、購入の行の「この購入をもとに登録」が引き継いだ登録の画面へ進むことを
    // 確かめる (issue 0058)。
    duplicate_from_the_list_and_the_detail(browser, fixture).await?;
    println!("duplicate: the row action opens the prefilled purchase form");

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
    browser.wait_text(PURCHASE_NAME).await?;
    browser.click_text(PURCHASE_NAME).await?;
    browser.wait_text("Save").await?;
    browser.click_text("Save").await?;
    // 保存の完了は通知で示す (画面には数えない)。
    browser.wait_text("Saved.").await?;
    browser.wait_route("/").await?;
    // 保存した抽出がホームの一覧に出る (抽出の保存まで)。
    browser.wait_text(PURCHASE_NAME).await?;
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
/// ヘッダーのハンバーガーメニューのログアウト (上の [`logout`]) とは別の入口 (設定の画面の
/// 文字ボタン) を確かめる (0044 のレビューの指摘)。
async fn logout_from_settings(browser: &E2eBrowser) -> Result<(), String> {
    browser.goto("/settings").await?;
    browser.wait_route("/settings").await?;
    browser.wait_text("Log out").await?;
    browser.click_text("Log out").await?;
    browser.wait_route("/login").await?;
    Ok(())
}

/// ヘッダーのハンバーガーメニューから 5 つの行き先へ移れることを確かめる (0047、0057)。
///
/// メニューはハンバーガーの下に開き、項目を押すと対応する経路へ移って閉じる。現在の経路と
/// 同じ項目を押したときは遷移せず、メニューだけ閉じる (履歴が増えないことで確かめる)。
/// 印を押すとホームへ戻り、ホームでは何もしない。
async fn nav_menu(browser: &E2eBrowser) -> Result<(), String> {
    // 5 つの行き先を順に開く (0057 で商品を外した)。
    for (label, route) in [
        ("Purchases", "/purchases"),
        ("Shops", "/shops"),
        ("Stats", "/stats"),
        ("Settings", "/settings"),
        ("Brews", "/"),
    ] {
        browser.click_selector("button[aria-label='Menu']").await?;
        browser.wait_selector(".menu").await?;
        browser.click_menu_item(label).await?;
        browser.wait_route(route).await?;
        browser.wait_menu_closed().await?;
    }

    // 現在の経路と同じ項目 (購入) を押したときは、遷移せずメニューだけ閉じる。
    browser.goto("/purchases").await?;
    browser.wait_route("/purchases").await?;
    let before = browser
        .eval_string("return String(window.history.length);")
        .await?;
    browser.click_selector("button[aria-label='Menu']").await?;
    browser.wait_selector(".menu").await?;
    browser.click_menu_item("Purchases").await?;
    browser.wait_menu_closed().await?;
    let after = browser
        .eval_string("return String(window.history.length);")
        .await?;
    if before != after {
        return Err(format!(
            "the current route item must not navigate but the history length changed from {before:?} to {after:?}"
        ));
    }
    browser.wait_route("/purchases").await?;

    // 印を押すとホームへ戻る。ホームでは何もしない (履歴が増えない)。
    browser.click_selector(".appbar .brandmark").await?;
    browser.wait_route("/").await?;
    let before = browser
        .eval_string("return String(window.history.length);")
        .await?;
    browser.click_selector(".appbar .brandmark").await?;
    let after = browser
        .eval_string("return String(window.history.length);")
        .await?;
    if before != after {
        return Err(format!(
            "the mark on the home must not navigate but the history length changed from {before:?} to {after:?}"
        ));
    }
    browser.wait_route("/").await?;

    // フォームの画面にはメニューと印を出さない (0054)。閉じると一覧へ戻る。
    browser.goto("/purchases").await?;
    browser.wait_route("/purchases").await?;
    browser.click_text("Log a purchase").await?;
    browser.wait_route("/purchases/new").await?;
    let has_menu = browser
        .eval_string(
            "return String(document.querySelector(\"button[aria-label='Menu']\") !== null);",
        )
        .await?;
    if has_menu.as_deref() != Some("false") {
        return Err(format!(
            "the form must not show the menu but it was {has_menu:?}"
        ));
    }
    let has_mark = browser
        .eval_string("return String(document.querySelector(\".appbar .brandmark\") !== null);")
        .await?;
    if has_mark.as_deref() != Some("false") {
        return Err(format!(
            "the form must not show the mark but it was {has_mark:?}"
        ));
    }
    browser
        .click_selector("button[aria-label='Cancel']")
        .await?;
    browser.wait_route("/purchases").await?;
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
            name: "PurchaseEdit",
            path: format!("/purchases/{}/edit", fixture.purchase_id),
            route: "/purchases/:id/edit",
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

/// 幅 375 px で検査するフォームの画面 (0048)。
struct FormScreen {
    /// 失敗の報告に使う名前。
    name: &'static str,
    /// 開くパス。
    path: String,
    /// 待つ `data-route`。
    route: &'static str,
    /// 統計の任意の期間か (「任意」を選ぶと入力欄が出る)。
    custom_period: bool,
}

/// フォームを持つ画面 (0048)。登録と編集の両方と、統計の任意の期間を並べる。
fn form_screens(fixture: &Fixture) -> Vec<FormScreen> {
    vec![
        FormScreen {
            name: "PurchaseNew",
            path: "/purchases/new".to_string(),
            route: "/purchases/new",
            custom_period: false,
        },
        FormScreen {
            name: "PurchaseEdit",
            path: format!("/purchases/{}/edit", fixture.purchase_id),
            route: "/purchases/:id/edit",
            custom_period: false,
        },
        FormScreen {
            name: "BrewNew",
            path: "/brews/new".to_string(),
            route: "/brews/new",
            custom_period: false,
        },
        FormScreen {
            name: "BrewEdit",
            path: format!("/brews/{}/edit", fixture.brew_id),
            route: "/brews/:id/edit",
            custom_period: false,
        },
        FormScreen {
            name: "ShopNew",
            path: "/shops/new".to_string(),
            route: "/shops/new",
            custom_period: false,
        },
        FormScreen {
            name: "ShopEdit",
            path: format!("/shops/{}/edit", fixture.shop_id),
            route: "/shops/:id/edit",
            custom_period: false,
        },
        FormScreen {
            name: "StatsCustom",
            path: "/stats".to_string(),
            route: "/stats",
            custom_period: true,
        },
    ]
}

/// 幅 375 px で各フォームが横にはみ出さず、最後の入力欄まで縦スクロールで到達できることを
/// 確かめる (0048)。
///
/// 症状は幅 375 px で計測したため、その幅で確かめる (390 px では `scrollWidth` が切り上げられて
/// 条件を満たし得る)。窓の大きさは OS の下限 (macOS の Chrome は 500 px) に丸められるため、
/// 表示の領域は CDP の端末の計測の上書きで 375 px にする。
async fn form_layouts(browser: &E2eBrowser, fixture: &Fixture) -> Result<(), String> {
    browser.set_window_size(375, 844).await?;
    browser.set_viewport_size(375, 844).await?;
    let result = check_form_layouts(browser, fixture).await;
    // 上書きを外して窓を元の大きさに戻す (この後の手順に影響させない)。
    let cleared = browser.clear_viewport_size().await;
    let resized = browser.set_window_size(390, 844).await;
    result.and(cleared).and(resized)
}

/// 各フォームの `.body` の横幅と、末尾まで縦スクロールしたときの最後の入力欄の位置を
/// 確かめる (0048)。
async fn check_form_layouts(browser: &E2eBrowser, fixture: &Fixture) -> Result<(), String> {
    for form in form_screens(fixture) {
        browser.goto(&form.path).await?;
        browser.wait_route(form.route).await?;
        if form.custom_period {
            // 統計の任意の期間は「任意」を選ぶと入力欄が出る (FR-18)。
            browser.click_text("Custom").await?;
            browser.wait_selector(".stats-custom .grid2").await?;
            // グラフの読み込みを待つ (読み込み中は `.body` の中身が短い)。
            browser.wait_selector(".chart-frame").await?;
        } else {
            // 編集の画面は値の読み込みを待つ (読み込み中は入力欄が無い)。
            browser.wait_selector(".form .field .box .in").await?;
        }
        let metrics = browser.eval_json(FORM_METRICS_SCRIPT).await?;
        check_form_metrics(&metrics, form.name)?;
    }
    Ok(())
}

/// `.body` の横幅と、末尾まで縦スクロールしたときの最後の入力欄の位置を測るスクリプト
/// (0048)。
///
/// 最後の入力欄は `input` と `textarea` のうち本文の順で最後のものにする。`scrollTop` を
/// 末尾にしてから入力欄を表示の位置に入れる (`scrollIntoView`)。統計の画面は入力欄の下に
/// グラフが続くため、末尾までのスクロールと入力欄の到達を別々に確かめられる必要がある。
const FORM_METRICS_SCRIPT: &str = r#"
const body = document.querySelector('.body');
if (!body) { return JSON.stringify({ error: 'the screen must have a .body' }); }
const inputs = body.querySelectorAll('input, textarea');
const last = inputs.length ? inputs[inputs.length - 1] : null;
body.scrollTop = body.scrollHeight;
const endScrollTop = body.scrollTop;
if (last && last.scrollIntoView) { last.scrollIntoView({ block: 'end' }); }
const bodyRect = body.getBoundingClientRect();
const lastRect = last ? last.getBoundingClientRect() : null;
return JSON.stringify({
  viewportWidth: window.innerWidth,
  viewportHeight: window.innerHeight,
  scrollWidth: body.scrollWidth,
  clientWidth: body.clientWidth,
  scrollHeight: body.scrollHeight,
  clientHeight: body.clientHeight,
  endScrollTop: endScrollTop,
  body: [bodyRect.top, bodyRect.bottom, bodyRect.left, bodyRect.right],
  last: lastRect ? [lastRect.top, lastRect.bottom, lastRect.left, lastRect.right] : null,
});
"#;

/// フォームの計測を確かめる (0048)。
fn check_form_metrics(metrics: &Value, name: &str) -> Result<(), String> {
    if let Some(error) = metrics.get("error") {
        return Err(format!("{name}: {error}"));
    }
    let viewport_width = metric_number(metrics, "viewportWidth")?;
    if (viewport_width - 375.0).abs() > 0.5 {
        return Err(format!(
            "{name}: the viewport must be 375 px but was {viewport_width}"
        ));
    }
    let scroll_width = metric_number(metrics, "scrollWidth")?;
    let client_width = metric_number(metrics, "clientWidth")?;
    if scroll_width > client_width {
        return Err(format!(
            "{name}: the form must not scroll horizontally but scrollWidth {scroll_width} exceeded clientWidth {client_width}"
        ));
    }
    let scroll_height = metric_number(metrics, "scrollHeight")?;
    let client_height = metric_number(metrics, "clientHeight")?;
    let end_scroll_top = metric_number(metrics, "endScrollTop")?;
    let max_scroll_top = (scroll_height - client_height).max(0.0);
    if (end_scroll_top - max_scroll_top).abs() > 1.0 {
        return Err(format!(
            "{name}: the body must scroll to the end but scrollTop was {end_scroll_top} of {max_scroll_top}"
        ));
    }
    let body = metric_rect(metrics, "body")?;
    let last = metric_rect(metrics, "last")?;
    if last[0] < body[0] - 1.0
        || last[1] > body[1] + 1.0
        || last[2] < body[2] - 1.0
        || last[3] > body[3] + 1.0
    {
        return Err(format!(
            "{name}: the last input must be inside the body after scrolling to the end: input {last:?} body {body:?}"
        ));
    }
    // 入力欄が表示の領域の中にあること (`.body` が表示の領域に縛られず伸びる退行を防ぐ)。
    let viewport_height = metric_number(metrics, "viewportHeight")?;
    if last[1] > viewport_height + 1.0 {
        return Err(format!(
            "{name}: the last input must be inside the viewport after scrolling to the end: input {last:?} viewport height {viewport_height}"
        ));
    }
    Ok(())
}

/// 購入の登録で写真を選ぶと、推測した商品名、生産者、タグがフォームに入ることを確かめる
/// (FR-19、0057)。
///
/// 推測の応答はページの `fetch` に差し込む。実環境の推測は Workers AI を呼ぶため E2E では
/// 使えない (0048 のテストのための仕掛け)。商品名は入力欄の値で、タグは Chip の文字で確かめる。
async fn suggestion_fills_the_form(browser: &E2eBrowser) -> Result<(), String> {
    browser.goto("/purchases/new").await?;
    browser.wait_route("/purchases/new").await?;
    browser.wait_selector(".form .field .box .in").await?;
    browser.eval_string(SUGGESTION_STUB_SCRIPT).await?;
    browser.click_text("Choose a photo").await?;
    // 写真の入力欄は押した時点で本文に足される (frontend/src/records/photo.rs)。
    browser
        .choose_file_selector("input[type=file]", &photo_path()?)
        .await?;
    wait_input_value(browser, "Product name", SUGGESTED_NAME).await?;
    wait_input_value(browser, "Producer", SUGGESTED_PRODUCER).await?;
    browser.wait_text(SUGGESTED_TAG).await?;
    Ok(())
}

/// 幅 390 px の購入の一覧の行と購入の詳細のヘッダーの「この購入をもとに登録」を押すと
/// `/purchases/new?from=<id>` へ進み、商品名、店、焙煎度が引き継がれることを確かめる (issue 0058)。
///
/// 1 列の表示にするため、窓を 390 px にしたうえで、表示の領域を CDP の端末の計測の上書きで
/// 390 px にする (`form_layouts` と同じ仕掛け。窓の大きさは OS の下限に丸められる)。
async fn duplicate_from_the_list_and_the_detail(
    browser: &E2eBrowser,
    fixture: &Fixture,
) -> Result<(), String> {
    browser.set_window_size(390, 844).await?;
    browser.set_viewport_size(390, 844).await?;
    let result = async {
        check_duplicate_from_the_list(browser, fixture).await?;
        check_duplicate_from_the_detail(browser, fixture).await
    }
    .await;
    let cleared = browser.clear_viewport_size().await;
    result.and(cleared)
}

/// 購入の一覧の行の操作から引き継いだ登録のフォームへ進む。
async fn check_duplicate_from_the_list(
    browser: &E2eBrowser,
    fixture: &Fixture,
) -> Result<(), String> {
    browser.goto("/purchases").await?;
    browser.wait_route("/purchases").await?;
    browser
        .wait_selector(".row .act .iconbtn[aria-label='Register from this purchase']")
        .await?;
    browser
        .click_selector(".row .act .iconbtn[aria-label='Register from this purchase']")
        .await?;
    wait_prefilled_purchase_form(browser, fixture, "the row action").await
}

/// 購入の詳細のヘッダーの操作から引き継いだ登録のフォームへ進む。
async fn check_duplicate_from_the_detail(
    browser: &E2eBrowser,
    fixture: &Fixture,
) -> Result<(), String> {
    browser
        .goto(&format!("/purchases/{}", fixture.purchase_id))
        .await?;
    browser.wait_route("/purchases/:id").await?;
    browser
        .wait_selector(".appbar .acts .iconbtn[aria-label='Register from this purchase']")
        .await?;
    browser
        .click_selector(".appbar .acts .iconbtn[aria-label='Register from this purchase']")
        .await?;
    wait_prefilled_purchase_form(browser, fixture, "the detail action").await
}

/// `/purchases/new?from=<id>` に着き、商品名、焙煎度、店が元の購入の値で入るまで待つ。
async fn wait_prefilled_purchase_form(
    browser: &E2eBrowser,
    fixture: &Fixture,
    action: &str,
) -> Result<(), String> {
    browser.wait_route("/purchases/new").await?;
    let path = browser.current_path().await?;
    let search = browser
        .eval_string("return window.location.search")
        .await?
        .unwrap_or_default();
    let expected = format!("?from={}", fixture.purchase_id);
    if path != "/purchases/new" || search != expected {
        return Err(format!(
            "{action} must open /purchases/new{expected} but opened {path}{search}"
        ));
    }
    browser.wait_text("Log a purchase").await?;
    wait_input_value(browser, "Product name", PURCHASE_NAME).await?;
    wait_input_value(browser, "Roast", PURCHASE_ROAST).await?;
    browser.wait_text(SHOP_NAME).await?;
    Ok(())
}

/// ラベルが `label` の入力欄 (`.form .field` の `.lbl` で探す) の値が期待の文字になるまで待つ。
async fn wait_input_value(browser: &E2eBrowser, label: &str, expected: &str) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(20);
    let script = format!(
        "const label = {};\
         const field = Array.from(document.querySelectorAll('.form .field')).find((field) => {{\
           const lbl = field.querySelector('.lbl');\
           return lbl && lbl.textContent.trim().startsWith(label);\
         }});\
         const input = field ? field.querySelector('input') : null;\
         return input ? String(input.value) : '';",
        serde_json::to_string(label).map_err(|error| error.to_string())?
    );
    loop {
        let value = browser.eval_string(&script).await?.unwrap_or_default();
        if value == expected {
            return Ok(());
        }
        if Instant::now() > deadline {
            return Err(format!(
                "the input {label} must have the value {expected:?} but had {value:?}"
            ));
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// 推測の導線に使う写真 (リポジトリにある PNG)。chromedriver は正規化したパスを求めるため、
/// `..` を含まない絶対パスにする。
fn photo_path() -> Result<PathBuf, String> {
    let path = repo_root().join("frontend/public/favicon.png");
    fs::canonicalize(&path).map_err(|error| format!("{} must exist: {error}", path.display()))
}

/// 推測 API の応答を差し替え、写真の選択のダイアログを開かないようにするスクリプト (0048)。
///
/// E2E の環境では Workers AI を呼べないため、推測の応答をページの `fetch` に差し込む。応答は
/// 商品の項目を平らに持つ (0057)。写真の選択は `<input type="file">` のクリックでダイアログを
/// 開くため、クリックを止めて入力欄を残し、あとから WebDriver でファイルを設定できるようにする。
const SUGGESTION_STUB_SCRIPT: &str = r#"
const realFetch = window.fetch.bind(window);
window.fetch = (input, init) => {
  const url = typeof input === 'string' ? input : (input && input.url) || '';
  if (url.includes('/api/purchase-suggestions')) {
    const body = JSON.stringify({
      name: 'E2E Suggested Beans', producer: 'E2E Suggested Producer', origin: null, region: null,
      process: null, variety: null, flavor_notes: ['E2E Suggested Tag'],
      roast: null, roast_date: null, price_amount: null, weight_grams: null
    });
    return Promise.resolve(new Response(body, { status: 200, headers: { 'Content-Type': 'application/json' } }));
  }
  return realFetch(input, init);
};
const originalClick = HTMLInputElement.prototype.click;
HTMLInputElement.prototype.click = function () {
  if (this.type === 'file') { window.__brewbookFileInput = this; return; }
  return originalClick.apply(this, arguments);
};
"#;

/// 計測の JSON から数の値を読む。
fn metric_number(metrics: &Value, key: &str) -> Result<f64, String> {
    metrics
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("the metrics must have {key}: {metrics}"))
}

/// 計測の JSON から矩形 (上、下、左、右) を読む。
fn metric_rect(metrics: &Value, key: &str) -> Result<[f64; 4], String> {
    let array = metrics
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("the metrics must have {key}: {metrics}"))?;
    let values: Option<Vec<f64>> = array.iter().map(Value::as_f64).collect();
    let values =
        values.ok_or_else(|| format!("the metrics must have numbers for {key}: {metrics}"))?;
    values
        .try_into()
        .map_err(|_| format!("the metrics must have four values for {key}: {metrics}"))
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
        screen: "PurchaseDetail",
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
    let purchases = value
        .get("purchases")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("the export must have the purchases array: {value}"))?;
    if !purchases
        .iter()
        .any(|purchase| purchase.get("name").and_then(Value::as_str) == Some(PURCHASE_NAME))
    {
        return Err(format!(
            "the export must contain the purchase {PURCHASE_NAME}: {value}"
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
