//! 実バックエンドと仮想認証器を使う統合テストのハーネス (0017)。
//!
//! 次の 3 つを組み合わせて、ログインから抽出の保存までを 1 本のテストで実行する (issue 0017 の方針)。
//!
//! - `wrangler dev`: テストコード入りの Web ビルド (`frontend/build/e2e-web`) を Static Assets として
//!   配信し、`/api/*` を Rust の処理に渡す。実際の画面はこのオリジンから読む。
//! - chromedriver: `flutter drive` が使うブラウザを起動する (`frontend:test-integration` と同じ)。
//! - `flutter drive`: Worker が配信する画面で integration_test を実行する。仮想認証器は
//!   ホスト側のドライバ (`frontend/test_driver/same_origin_test.dart`) が付ける。
//!
//! 利用者、登録用トークン、店、商品、購入はローカルの D1 に直接投入する (0005 の下ごしらえ)。
//! テスト名の `wrangler_` は、`wrangler dev` を起動するテストを `backend:test` が名前で除外するための規約。
//! このテストは `frontend:test-same-origin` が実行する (Web ビルドと chromedriver を要するため)。

mod support;

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use support::seed::{self, Seed};
use support::{free_port, DevServer};

/// 下ごしらえに使う時刻 (ISO 8601 UTC の固定長)。
const CREATED: &str = "2026-09-01T00:00:00.000Z";
const FUTURE: &str = "2099-01-01T00:00:00.000Z";
/// 下ごしらえした商品の名前 (Dart のテストと同じ値)。
const PRODUCT_NAME: &str = "E2E Product";
/// 下ごしらえした購入の購入日。
const PURCHASED_ON: &str = "2026-09-21";

/// `flutter drive` のビルドと実行と、パスキーのやり取りを待つ上限。
const FLUTTER_DRIVE_TIMEOUT: Duration = Duration::from_secs(900);
/// 失敗したときに報告するログの末尾の行数。
const LOG_TAIL_LINES: usize = 60;

#[test]
fn wrangler_web_routes_registration_login_and_brew_save_ok() {
    let frontend = frontend_dir();
    let assets = frontend.join("build/e2e-web");
    let main_js = assets.join("main.dart.js");
    assert!(
        main_js.is_file(),
        "the E2E web build must exist at {} (run `mise run frontend:build-e2e`)",
        assets.display()
    );
    // 配信されるビルドがテストコード入りであることを、テストだけが持つ文字列で確かめる
    // (配布用のビルドを配信してしまうと、テストが時間切れになるまで気付けない)。
    let build = std::fs::read_to_string(&main_js)
        .map_err(|error| format!("failed to read {}: {error}", main_js.display()))
        .expect("the E2E web build must be readable");
    assert!(
        build.contains(PRODUCT_NAME),
        "{} must contain the E2E test code (run `mise run frontend:build-e2e`)",
        main_js.display()
    );

    // 下ごしらえ: 利用者、登録用トークン、店、商品、購入。
    // 購入は抽出の保存の画面が選ぶため、購入まで入れておく。
    let mut seed = Seed::new();
    let user = seed::user_id(1);
    seed.user(&user, "E2E の利用者", CREATED);
    let token = seed.registration_token(&user, FUTURE);
    let shop = seed.shop(&user, "E2E の店", None, CREATED, CREATED, None);
    let product = seed.product(&user, PRODUCT_NAME, CREATED, CREATED, None);
    seed.purchase(
        &user,
        &product.id,
        Some(&shop.id),
        PURCHASED_ON,
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

    // ブラウザを起動する chromedriver。`flutter drive` は 4444 を既定にするため、空きポートを渡す
    // (同じ check の中で `frontend:test-integration` が 4444 を使うことがある)。
    let driver_port = free_port().expect("a free port for chromedriver must be found");
    let mut chromedriver = start_chromedriver(driver_port);

    // Flutter のルーティングのパスを直接開く。`/api/*` は Worker、それ以外は Static Assets が返す。
    let launch_url = format!("{}/register?token={token}", server.localhost_url());
    let result = run_flutter_drive(&frontend, driver_port, &launch_url);

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

/// 出力の末尾を読む (失敗の報告に使う)。Worker はリクエスト 1 件ごとに経路名と状態コードを出す。
fn output_tail(lines: &[String]) -> String {
    let start = lines.len().saturating_sub(LOG_TAIL_LINES);
    lines[start..].join("\n")
}

/// リポジトリの `frontend` ディレクトリ。
fn frontend_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../frontend")
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

/// `flutter drive` を実行し、終了を待つ。失敗した場合はログの末尾を報告する。
fn run_flutter_drive(frontend: &Path, driver_port: u16, launch_url: &str) -> Result<(), String> {
    let log_path = std::env::temp_dir().join(format!(
        "brewbook-e2e-flutter-drive-{}.log",
        std::process::id()
    ));
    let log = File::create(&log_path)
        .map_err(|error| format!("failed to create {}: {error}", log_path.display()))?;
    let stdout = log
        .try_clone()
        .map_err(|error| format!("failed to clone the log file: {error}"))?;

    let mut child = Command::new("flutter")
        .args([
            "drive",
            "--driver=test_driver/same_origin_test.dart",
            "--target=integration_test/same_origin_test.dart",
            "-d",
            "web-server",
            "--browser-name=chrome",
            "--headless",
            &format!("--driver-port={driver_port}"),
            &format!("--web-launch-url={launch_url}"),
        ])
        .current_dir(frontend)
        // AI エージェント検出を無効にする (wrangler と同じ理由)。
        .env_remove("OPENCODE")
        .env_remove("AGENT")
        .env_remove("AI_AGENT")
        .env_remove("CLAUDECODE")
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(log))
        .spawn()
        .map_err(|error| {
            format!("failed to start flutter drive (run this test with `mise run frontend:test-same-origin`): {error}")
        })?;

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let tail = log_tail(&log_path);
                if status.success() {
                    println!("flutter drive passed. the log is at {}", log_path.display());
                    return Ok(());
                }
                return Err(format!(
                    "flutter drive exited with {status}\nthe log is at {}\n{tail}",
                    log_path.display()
                ));
            }
            Ok(None) if started.elapsed() > FLUTTER_DRIVE_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "flutter drive did not finish within {}s\nthe log is at {}\n{}",
                    FLUTTER_DRIVE_TIMEOUT.as_secs(),
                    log_path.display(),
                    log_tail(&log_path)
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(500)),
            Err(error) => return Err(format!("failed to check flutter drive: {error}")),
        }
    }
}

/// ログの末尾を読む (失敗の報告に使う)。
fn log_tail(path: &Path) -> String {
    let mut text = String::new();
    if let Ok(mut file) = File::open(path) {
        let _ = file.read_to_string(&mut text);
    }
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(LOG_TAIL_LINES);
    lines[start..].join("\n")
}
