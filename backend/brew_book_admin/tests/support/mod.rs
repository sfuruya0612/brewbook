//! 管理者 Worker の結合テストのハーネス (0001 のハーネスと同じ方式)。
//!
//! `wrangler dev` を子プロセスとして起動し、応答を待ってからテストを実行し、終了時に停止する。
//! スキーマは利用者向けの Worker のマイグレーションで作る (管理者 Worker はマイグレーションを
//! 持たない。ADR-0002)。ローカルの D1 は状態ディレクトリを共有するため、利用者向けの Worker の
//! `wrangler dev` も同じデータベースを見る。
//!
//! 利用者向けの Worker の `wrangler dev` も起動する。再発行したトークンが無効になること (FR-17) の
//! 確認が、登録の経路 (利用者向けの Worker) を必要とするためである。
//!
//! 成功指標の照合 (経路の台帳とテストの識別子) もここが持つ。管理者 API はアプリ内の認証を
//! 持たない (ADR-0008) ため、種別は正常系と入力不正 400 の 2 つだけである。
//!
//! `wrangler dev` は子として `workerd` と esbuild を起動する。停止はプロセスグループごとに
//! 行い、孫が孤児として残らないようにする (0019)。

#![allow(dead_code)] // ハーネスは複数のテストクレートで共有するため、各クレートから見て未使用の項目がある

use std::io::{BufRead, BufReader, Read};
use std::net::TcpListener;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::thread;
use std::time::{Duration, Instant};

/// 正常系のテストの種別。
pub const KIND_OK: &str = "ok";
/// 入力不正 400 のテストの種別。
pub const KIND_INVALID_INPUT_400: &str = "invalid_input_400";

/// 起動を待つ上限。初回は worker-build のビルドを含むため長めにする。
const READY_TIMEOUT: Duration = Duration::from_secs(600);
const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// `stop` がプロセスグループへ SIGTERM を送ってから、SIGKILL に切り替えるまでの待ち時間。
const STOP_TIMEOUT: Duration = Duration::from_secs(5);

/// 結合テストのスイートが経路ごとに持つテストの種別。
pub struct SuiteEntry {
    pub route: &'static str,
    pub kinds: &'static [&'static str],
}

/// このスイートが持つテストの種別。`tests/wrangler_admin_api.rs` のテストと対応する。
pub const SUITE: &[SuiteEntry] = &[
    SuiteEntry {
        route: "users_list",
        kinds: &[KIND_OK],
    },
    SuiteEntry {
        route: "users_create",
        kinds: &[KIND_OK, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "tokens_create",
        kinds: &[KIND_OK],
    },
];

/// 台帳とスイートが一致するかを検査する (PRD の成功指標の管理者 Worker の分)。
pub fn admin_suite_covers_ledger() -> Result<(), String> {
    covers(brew_book_admin::routes::ROUTES, SUITE)
}

/// 経路の台帳とスイートのテスト種別が一致するかを検査する。
///
/// 台帳の全経路が同じ名前のスイートの項目を持ち、スイートの種別が必要な種別 (正常系、
/// 入力不正 400) と過不足なく一致することを確認する。
pub fn covers(
    routes: &[brew_book_core::routes::Route],
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
        let requirements = brew_book_core::routes::test_requirements(route);
        let mut expected: Vec<&str> = Vec::new();
        // 正常系を CI で実行できない経路は正常系の種別を要求しない (管理者の経路は全て CI。FR-19)。
        if requirements.ok == brew_book_core::routes::OkTest::Ci {
            expected.push(KIND_OK);
        }
        if requirements.unauthenticated_401 {
            expected.push("unauthenticated_401");
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

/// テストの間だけ残す状態ディレクトリ。最後の参照が落ちたときに消す。
pub struct Persist(Arc<PersistDir>);

struct PersistDir(PathBuf);

impl Drop for PersistDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl Persist {
    /// 状態ディレクトリを作る。
    pub fn new() -> Result<Self, String> {
        let path =
            std::env::temp_dir().join(format!("brewbook-admin-dev-server-{}", std::process::id()));
        std::fs::create_dir_all(&path)
            .map_err(|error| format!("failed to create {}: {error}", path.display()))?;
        Ok(Self(Arc::new(PersistDir(path))))
    }

    pub fn path(&self) -> &Path {
        &self.0 .0
    }
}

/// 管理者 Worker と利用者向けの Worker の `wrangler dev` と、共有の状態ディレクトリ。
pub struct Servers {
    pub admin: DevServer,
    pub app: DevServer,
    persist: Persist,
}

impl Servers {
    /// 利用者向けの Worker のマイグレーションを適用し、利用者向けの Worker と管理者 Worker を起動する。
    ///
    /// 管理者 Worker の `APP_ORIGIN` には、起動した利用者向けの Worker の URL を渡す
    /// (登録用リンクがこの Worker を指す。ADR-0008)。
    pub fn start() -> Result<Self, String> {
        let persist = Persist::new()?;
        apply_migrations(persist.path())?;
        ensure_app_assets_dir()?;
        let app = DevServer::start(
            &manifest_dir().join("../brew_book"),
            persist.path(),
            "/api/__ready",
            &[],
        )?;
        let admin = DevServer::start(
            &manifest_dir(),
            persist.path(),
            "/",
            &[("APP_ORIGIN", &app.base_url())],
        )?;
        Ok(Self {
            admin,
            app,
            persist,
        })
    }
}

/// `wrangler dev` の子プロセスと、その出力。
pub struct DevServer {
    child: Child,
    port: u16,
    persist_dir: PathBuf,
    output_lines: Arc<Mutex<Vec<String>>>,
}

impl DevServer {
    /// `wrangler dev` を起動し、応答を待つ。`manifest_dir` は wrangler.toml のあるディレクトリ。
    /// `ready_path` は起動の確認に GET するパス。`vars` は `--var` で渡す vars。
    pub fn start(
        manifest_dir: &Path,
        persist_dir: &Path,
        ready_path: &str,
        vars: &[(&str, &str)],
    ) -> Result<Self, String> {
        let port = free_port()?;
        let mut command = Command::new("wrangler");
        command
            .arg("dev")
            // リモートのバインディング (利用者向けの Worker の AI) を無効にして起動する。AI
            // バインディングは起動時にリモートのプロキシのセッションを開くため、これが無いと
            // ログインの無い CI や、アカウントを 1 つに選べない環境では `wrangler dev` が起動しない
            // (このスイートは利用者向けの Worker の `wrangler dev` も起動する。ADR-0016、issue 0034)。
            .arg("--local")
            .arg("--ip")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            .arg("--persist-to")
            .arg(persist_dir)
            .current_dir(manifest_dir)
            .env("WRANGLER_SEND_METRICS", "false")
            // AI エージェント検出を無効にする。検出されると wrangler はログを標準出力ではなく
            // Local Explorer の観測ストアに出すため、テストが出力を読めない。
            .env_remove("OPENCODE")
            .env_remove("AGENT")
            .env_remove("AI_AGENT")
            .env_remove("CLAUDECODE")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        // 孫 (workerd、esbuild) まで確実に停止できるように、子を新しいプロセスグループにする (0019)。
        command.process_group(0);
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
            persist_dir: persist_dir.to_path_buf(),
            output_lines,
        };
        if let Err(error) = server.wait_until_ready(ready_path) {
            server.stop();
            return Err(error);
        }
        Ok(server)
    }

    /// この Worker の基底 URL。
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// 使用中のポート。
    pub fn port(&self) -> u16 {
        self.port
    }

    /// `wrangler d1 execute` でローカルの D1 に SQL を実行する (下ごしらえに使う)。
    pub fn execute_sql(&self, sql: &str) -> Result<(), String> {
        let output = d1_execute_command(&manifest_dir(), &self.persist_dir)
            .arg("--command")
            .arg(sql)
            .output()
            .map_err(|error| format!("failed to run wrangler d1 execute: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "wrangler d1 execute failed with {}\nstdout:\n{}\nstderr:\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    }

    /// `wrangler d1 execute --json` でローカルの D1 から行を引く。
    pub fn query_rows(&self, sql: &str) -> Result<Vec<serde_json::Value>, String> {
        let output = run_d1_execute_json(&manifest_dir(), &self.persist_dir, sql)?;
        let parsed: serde_json::Value = serde_json::from_str(&output)
            .map_err(|error| format!("the d1 output must be JSON but was {output}: {error}"))?;
        let rows = parsed
            .get(0)
            .and_then(|result| result.get("results"))
            .and_then(|results| results.as_array())
            .cloned();
        rows.ok_or_else(|| format!("the d1 output has no rows: {output}"))
    }

    /// D1 から 1 つの整数を引く。先頭の行の先頭の列を整数として読む。
    pub fn query_int(&self, sql: &str) -> Result<i64, String> {
        let rows = self.query_rows(sql)?;
        rows.first()
            .and_then(|row| row.as_object())
            .and_then(|row| row.values().next())
            .and_then(|value| value.as_i64())
            .ok_or_else(|| format!("the d1 output has no integer: {rows:?}"))
    }

    /// 子プロセスの出力 (stdout と stderr の全行) を返す。
    pub fn output(&self) -> Vec<String> {
        self.output_lines
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// HTTP が応答するまで待つ。子プロセスが先に終了したら失敗にする。
    fn wait_until_ready(&mut self, ready_path: &str) -> Result<(), String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|error| format!("failed to build the HTTP client: {error}"))?;
        let url = format!("{}{ready_path}", self.base_url());
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
    ///
    /// `wrangler` は子として `workerd` と esbuild を起動する。`Child::kill` (SIGKILL) だけでは
    /// `wrangler` が後始末できず、孫が孤児として残ってしまう (0019)。プロセスグループごと
    /// SIGTERM で止め、`STOP_TIMEOUT` を過ぎても残っていれば SIGKILL で確実に止める。
    pub fn stop(&mut self) {
        let process_group = self.child.id() as libc::pid_t;
        // 起動に失敗した場合など、既に子が終了していても killpg は無害 (ESRCH になるだけ)。
        unsafe { libc::killpg(process_group, libc::SIGTERM) };
        let deadline = Instant::now() + STOP_TIMEOUT;
        while Instant::now() < deadline {
            match self.child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => thread::sleep(POLL_INTERVAL),
                Err(_) => break,
            }
        }
        unsafe { libc::killpg(process_group, libc::SIGKILL) };
        let _ = self.child.wait();
    }

    /// テスト用: 子プロセスが属するプロセスグループの ID。
    ///
    /// `process_group(0)` で起動するため、子プロセスの ID と同じになる。
    pub fn process_group_id(&self) -> u32 {
        self.child.id()
    }
}

/// テスト用: プロセスグループにプロセスが残っているかを確認する。
///
/// 停止の後に `workerd` などの孫が残っていないことの検証に使う (0019)。
pub fn process_group_exists(process_group_id: u32) -> bool {
    unsafe { libc::killpg(process_group_id as libc::pid_t, 0) == 0 }
}

impl Drop for DevServer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// 共有する `wrangler dev` の置き場の 1 件。借用が残っている間だけ生かすため Weak で持つ。
type SharedSlot = (&'static str, Weak<Mutex<Option<Servers>>>);

/// 共有する `wrangler dev` の置き場。
static SHARED: Mutex<Vec<SharedSlot>> = Mutex::new(Vec::new());

/// テストファイルで共有する `wrangler dev` を借りる。`name` ごとに 1 つ起動する。
/// 最後の借用が落ちると `Servers` が落ちて子プロセスが停止する。
pub fn shared_servers(
    name: &'static str,
    start: impl FnOnce() -> Result<Servers, String>,
) -> Result<ServerLease, String> {
    let mut slot = SHARED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(servers) = slot
        .iter()
        .find(|(key, _)| *key == name)
        .and_then(|(_, weak)| weak.upgrade())
    {
        return Ok(ServerLease(servers));
    }
    let servers = start()?;
    let servers = Arc::new(Mutex::new(Some(servers)));
    slot.retain(|(key, _)| *key != name);
    slot.push((name, Arc::downgrade(&servers)));
    Ok(ServerLease(servers))
}

/// 共有のサーバーの借用。借用中は他のテストを待たせる。
pub struct ServerLease(Arc<Mutex<Option<Servers>>>);

impl ServerLease {
    /// サーバーと状態ディレクトリを使う。借用の間は他のテストを待たせる。
    pub fn use_servers<T>(&self, action: impl FnOnce(&Servers) -> T) -> T {
        let guard: MutexGuard<'_, Option<Servers>> = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let servers = guard
            .as_ref()
            .expect("the shared servers must live while the lease is held");
        action(servers)
    }
}

/// 子プロセスの出力を 1 行ずつ集める。
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

/// このクレート (`backend/brew_book_admin`) のディレクトリ。
pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `wrangler dev` は Static Assets のディレクトリが無いと起動しないため、先に作る。
pub fn ensure_app_assets_dir() -> Result<(), String> {
    // クレートのルートは backend/brew_book_admin のため、リポジトリ直下の frontend へは 2 段上る
    // (0045 のレビューの指摘)。
    let dir = manifest_dir().join("../../frontend/target/dx/brew_book_frontend/release/web/public");
    std::fs::create_dir_all(&dir)
        .map_err(|error| format!("failed to create {}: {error}", dir.display()))
}

/// 利用者向けの Worker のマイグレーションを、共有の状態ディレクトリの D1 に適用する。
///
/// 管理者 Worker はマイグレーションを持たないため、スキーマは利用者向けの Worker の
/// マイグレーションだけが作る (ADR-0002)。適用は利用者向けの Worker の設定から行う。
pub fn apply_migrations(persist_dir: &Path) -> Result<(), String> {
    let migrations = manifest_dir().join("../brew_book/migrations");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&migrations)
        .map_err(|error| format!("failed to read {}: {error}", migrations.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "sql"))
        .collect();
    files.sort();
    if files.is_empty() {
        return Err(format!("no migrations in {}", migrations.display()));
    }
    for file in files {
        let status = d1_execute_command(&manifest_dir().join("../brew_book"), persist_dir)
            .arg("--file")
            .arg(&file)
            .status()
            .map_err(|error| format!("failed to run wrangler d1 execute: {error}"))?;
        if !status.success() {
            return Err(format!(
                "wrangler d1 execute --file {} failed with {status}",
                file.display()
            ));
        }
    }
    Ok(())
}

/// `wrangler d1 execute --json` を実行し、標準出力を返す。
fn run_d1_execute_json(
    manifest_dir: &Path,
    persist_dir: &Path,
    sql: &str,
) -> Result<String, String> {
    let output = d1_execute_command(manifest_dir, persist_dir)
        .arg("--command")
        .arg(sql)
        .arg("--json")
        .output()
        .map_err(|error| format!("failed to run wrangler d1 execute: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.status.success() {
        return Err(format!(
            "wrangler d1 execute failed with {}\nstdout:\n{stdout}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(stdout)
}

/// `wrangler d1 execute` の共通の引数を組み立てる。
fn d1_execute_command(manifest_dir: &Path, persist_dir: &Path) -> Command {
    let mut command = Command::new("wrangler");
    command
        .args(["d1", "execute", "DB", "--local", "--persist-to"])
        .arg(persist_dir)
        .current_dir(manifest_dir)
        .env("WRANGLER_SEND_METRICS", "false")
        .env_remove("OPENCODE")
        .env_remove("AGENT")
        .env_remove("AI_AGENT")
        .env_remove("CLAUDECODE");
    command
}

/// OS に空きポートを 1 つ選ばせる。
pub fn free_port() -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("failed to find a free port: {error}"))?;
    listener
        .local_addr()
        .map(|address| address.port())
        .map_err(|error| format!("failed to read the local address: {error}"))
}
