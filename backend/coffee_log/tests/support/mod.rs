//! 結合テストのハーネス。
//!
//! `wrangler dev` を子プロセスとして起動し (ローカルの状態ディレクトリを指定する)、
//! マイグレーションを適用し、応答を待ってからテストを実行し、終了時に停止する (ADR-0001、ADR-0009)。

#![allow(dead_code)] // ハーネスは複数のテストクレートで共有するため、各クレートから見て未使用の項目がある

use std::io::{BufRead, BufReader, Read};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// 起動を待つ上限。初回は worker-build のビルドを含むため長めにする。
const READY_TIMEOUT: Duration = Duration::from_secs(600);
const POLL_INTERVAL: Duration = Duration::from_millis(500);
const OUTPUT_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// 起動ごとの連番。同じテストバイナリ内で複数のサーバーを起動しても状態ディレクトリが衝突しないようにする。
static START_COUNTER: AtomicU64 = AtomicU64::new(0);

/// 正常系のテストの種別。
pub const KIND_OK: &str = "ok";
/// 未認証 401 のテストの種別。
pub const KIND_UNAUTHENTICATED_401: &str = "unauthenticated_401";
/// 入力不正 400 のテストの種別。
pub const KIND_INVALID_INPUT_400: &str = "invalid_input_400";

/// 結合テストのスイートが経路ごとに持つテストの種別。
///
/// 経路を実装した issue が、経路を台帳へ追加すると同時にここへ追加する (PRD の成功指標の照合)。
pub struct SuiteEntry {
    pub route: &'static str,
    pub kinds: &'static [&'static str],
}

/// このスイートが持つテストの種別。経路は 0005 以降が追加する。
pub const SUITE: &[SuiteEntry] = &[];

/// 台帳とスイートが一致するかを検査する。
pub fn suite_covers_ledger() -> Result<(), String> {
    covers(coffee_log_core::routes::ROUTES, SUITE)
}

/// 経路の台帳とスイートのテスト種別が一致するかを検査する。
///
/// 台帳の全経路が同じ名前のスイートの項目を持ち、スイートの種別が必要な種別 (正常系、
/// 未認証 401、入力不正 400) と過不足なく一致することを確認する。
pub fn covers(
    routes: &[coffee_log_core::routes::Route],
    suite: &[SuiteEntry],
) -> Result<(), String> {
    let mut route_names: Vec<&str> = routes.iter().map(|route| route.name).collect();
    let mut suite_names: Vec<&str> = suite.iter().map(|entry| entry.route).collect();
    route_names.sort_unstable();
    suite_names.sort_unstable();
    if route_names != suite_names {
        return Err(format!(
            "the ledger has routes {route_names:?} but the suite has {suite_names:?}"
        ));
    }

    for route in routes {
        let entry = suite
            .iter()
            .find(|entry| entry.route == route.name)
            .expect("the route names match above");
        let requirements = coffee_log_core::routes::test_requirements(route);
        let mut expected: Vec<&str> = Vec::new();
        if requirements.ok {
            expected.push(KIND_OK);
        }
        if requirements.unauthenticated_401 {
            expected.push(KIND_UNAUTHENTICATED_401);
        }
        if requirements.invalid_input_400 {
            expected.push(KIND_INVALID_INPUT_400);
        }
        let mut actual: Vec<&str> = entry.kinds.to_vec();
        expected.sort_unstable();
        actual.sort_unstable();
        if actual != expected {
            return Err(format!(
                "route {} must have tests {expected:?} but the suite has {actual:?}",
                route.name
            ));
        }
    }
    Ok(())
}

/// `wrangler dev` の子プロセスと、その出力。
pub struct DevServer {
    child: Child,
    port: u16,
    persist_dir: PathBuf,
    output_lines: Arc<Mutex<Vec<String>>>,
}

impl DevServer {
    /// `wrangler dev` を起動し、マイグレーションを適用し、応答を待つ。
    pub fn start() -> Result<Self, String> {
        Self::start_with_vars(&[])
    }

    /// `--var` で渡す vars を指定して `wrangler dev` を起動する。
    /// 本番の vars に無い値をテストから注入するために使う。
    pub fn start_with_vars(vars: &[(&str, &str)]) -> Result<Self, String> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let sequence = START_COUNTER.fetch_add(1, Ordering::Relaxed);
        let persist_dir = std::env::temp_dir().join(format!(
            "coffee-log-dev-server-{}-{}",
            std::process::id(),
            sequence
        ));
        std::fs::create_dir_all(&persist_dir)
            .map_err(|error| format!("failed to create {}: {error}", persist_dir.display()))?;
        let port = free_port()?;

        let mut command = Command::new("wrangler");
        command
            .arg("dev")
            .arg("--ip")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            .arg("--persist-to")
            .arg(&persist_dir)
            .current_dir(&manifest_dir)
            .env("WRANGLER_SEND_METRICS", "false")
            // AI エージェント検出を無効にする。検出されると wrangler はログを標準出力ではなく
            // Local Explorer の観測ストアに出すため、テストが出力を読めない。
            .env_remove("OPENCODE")
            .env_remove("AGENT")
            .env_remove("AI_AGENT")
            .env_remove("CLAUDECODE")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in vars {
            command.arg("--var").arg(format!("{key}:{value}"));
        }
        let mut child = command
            .spawn()
            .map_err(|error| format!("failed to start wrangler dev: {error}"))?;

        let output_lines = Arc::new(Mutex::new(Vec::new()));
        capture_output(child.stdout.take(), Arc::clone(&output_lines));
        capture_output(child.stderr.take(), Arc::clone(&output_lines));

        let mut server = Self {
            child,
            port,
            persist_dir,
            output_lines,
        };
        if let Err(error) = server.apply_migrations() {
            server.stop();
            return Err(error);
        }
        if let Err(error) = server.wait_until_ready() {
            server.stop();
            return Err(error);
        }
        Ok(server)
    }

    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// 子プロセスの出力 (stdout と stderr の全行) を返す。
    pub fn output(&self) -> Vec<String> {
        self.output_lines
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// 子プロセスの出力から `needle` を含む最初の行を返す。現れるまで `timeout` だけ待つ。
    /// ログは応答の後に届くため、応答の直後に出力を読むと間に合わない。
    pub fn wait_for_line(&self, needle: &str, timeout: Duration) -> Option<String> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(line) = self.output().into_iter().find(|line| line.contains(needle)) {
                return Some(line);
            }
            if Instant::now() >= deadline {
                return None;
            }
            thread::sleep(OUTPUT_POLL_INTERVAL);
        }
    }

    /// `wrangler d1 migrations apply` でローカルの D1 にマイグレーションを適用する。
    ///
    /// D1 のバインディングは 0003 が追加する。バインディングが無い間は適用をスキップする。
    fn apply_migrations(&mut self) -> Result<(), String> {
        let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("wrangler.toml");
        let config = std::fs::read_to_string(&config_path)
            .map_err(|error| format!("failed to read {}: {error}", config_path.display()))?;
        if !has_d1_binding(&config) {
            self.record("migrations skipped: no [[d1_databases]] in wrangler.toml".to_owned());
            return Ok(());
        }
        let status = Command::new("wrangler")
            .args(["d1", "migrations", "apply", "DB", "--local", "--persist-to"])
            .arg(&self.persist_dir)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .env("WRANGLER_SEND_METRICS", "false")
            .status()
            .map_err(|error| format!("failed to run wrangler d1 migrations apply: {error}"))?;
        if !status.success() {
            return Err(format!("wrangler d1 migrations apply failed with {status}"));
        }
        Ok(())
    }

    /// HTTP が応答するまで待つ。子プロセスが先に終了したら失敗にする。
    fn wait_until_ready(&mut self) -> Result<(), String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|error| format!("failed to build the HTTP client: {error}"))?;
        let url = format!("{}/api/__ready", self.base_url());
        let deadline = Instant::now() + READY_TIMEOUT;
        while Instant::now() < deadline {
            if let Some(status) = self
                .child
                .try_wait()
                .map_err(|error| format!("failed to check wrangler dev: {error}"))?
            {
                return Err(format!(
                    "wrangler dev exited before ready with {status}\n{}",
                    self.output().join("\n")
                ));
            }
            if client.get(&url).send().is_ok() {
                return Ok(());
            }
            thread::sleep(POLL_INTERVAL);
        }
        Err(format!(
            "wrangler dev did not answer within {}s\n{}",
            READY_TIMEOUT.as_secs(),
            self.output().join("\n")
        ))
    }

    /// 子プロセスを停止する (Drop でも呼ばれる)。
    pub fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.persist_dir);
    }

    fn record(&self, line: String) {
        self.output_lines
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(line);
    }
}

impl Drop for DevServer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// 子プロセスの出力を 1 行ずつ集める。読み取りに失敗したらその旨を出力に残す。
fn capture_output<R: Read + Send + 'static>(reader: Option<R>, sink: Arc<Mutex<Vec<String>>>) {
    if let Some(reader) = reader {
        thread::spawn(move || {
            let mut lines = BufReader::new(reader).lines();
            loop {
                match lines.next() {
                    Some(Ok(line)) => sink
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .push(line),
                    Some(Err(error)) => {
                        sink.lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner())
                            .push(format!("failed to read the wrangler output: {error}"));
                        break;
                    }
                    None => break,
                }
            }
        });
    }
}

/// `wrangler.toml` が D1 のバインディングを定義しているかを判定する。
/// TOML のテーブル見出しは前後の空白を許すため、空白を除いて比較する。
fn has_d1_binding(config: &str) -> bool {
    config
        .lines()
        .any(|line| line.split_whitespace().collect::<String>() == "[[d1_databases]]")
}

/// OS に空きポートを 1 つ選ばせる。wrangler が bind するまでの間に他プロセスが使う可能性は残る。
fn free_port() -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("failed to find a free port: {error}"))?;
    listener
        .local_addr()
        .map(|address| address.port())
        .map_err(|error| format!("failed to read the local address: {error}"))
}
