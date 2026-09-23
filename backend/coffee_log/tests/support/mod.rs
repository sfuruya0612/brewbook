//! 結合テストのハーネス。
//!
//! `wrangler dev` を子プロセスとして起動し (ローカルの状態ディレクトリを指定する)、
//! マイグレーションを適用し、応答を待ってからテストを実行し、終了時に停止する (ADR-0001、ADR-0009)。
//!
//! サーバーの起動は 1 つのテストファイルにつき 1 回にしたいので、[`shared_server`] で借りる形の
//! 共有を持つ。最後の借用が落ちたときに子プロセスを停止し、状態ディレクトリを消す。

#![allow(dead_code)] // ハーネスは複数のテストクレートで共有するため、各クレートから見て未使用の項目がある

use std::io::{BufRead, BufReader, Read};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::thread;
use std::time::{Duration, Instant};

pub mod cdp;
pub mod http;
pub mod seed;

/// 起動を待つ上限。初回は worker-build のビルドを含むため長めにする。
const READY_TIMEOUT: Duration = Duration::from_secs(600);
const POLL_INTERVAL: Duration = Duration::from_millis(500);
const OUTPUT_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// 起動ごとの連番。同じテストバイナリ内で複数のサーバーを起動しても状態ディレクトリが衝突しないようにする。
static START_COUNTER: AtomicU64 = AtomicU64::new(0);

/// R2 の操作の一時ファイルの連番。同じテストバイナリ内の並行するテストで衝突しないようにする。
static R2_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

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

/// このスイートが持つテストの種別。0005 が認証の 10 経路、0006 が店と商品とタグの 13 経路、
/// 0007 が購入と抽出の 12 経路、0008 がサジェストの 1 経路、0009 が購入の写真の 4 経路、
/// 0010 が統計と評価の推移の 4 経路を追加する。
pub const SUITE: &[SuiteEntry] = &[
    SuiteEntry {
        route: "auth_register_begin",
        kinds: &[KIND_OK, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "auth_register_complete",
        kinds: &[KIND_OK, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "auth_login_begin",
        kinds: &[KIND_OK],
    },
    SuiteEntry {
        route: "auth_login_complete",
        kinds: &[KIND_OK, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "auth_logout",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401],
    },
    SuiteEntry {
        route: "passkeys_list",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401],
    },
    SuiteEntry {
        route: "passkeys_begin",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401],
    },
    SuiteEntry {
        route: "passkeys_complete",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "passkeys_rename",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "passkeys_delete",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401],
    },
    SuiteEntry {
        route: "shops_list",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "shops_create",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "shops_get",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401],
    },
    SuiteEntry {
        route: "shops_update",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "shops_archive",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401],
    },
    SuiteEntry {
        route: "shops_unarchive",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401],
    },
    SuiteEntry {
        route: "products_list",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "products_create",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "products_get",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401],
    },
    SuiteEntry {
        route: "products_update",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "products_archive",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401],
    },
    SuiteEntry {
        route: "products_unarchive",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401],
    },
    SuiteEntry {
        route: "flavor_tags_list",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401],
    },
    SuiteEntry {
        route: "purchases_list",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "purchases_create",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "purchases_get",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401],
    },
    SuiteEntry {
        route: "purchases_update",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "purchases_archive",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401],
    },
    SuiteEntry {
        route: "purchases_unarchive",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401],
    },
    SuiteEntry {
        route: "purchases_photo_upload_url",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "purchases_photo_complete",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "purchases_photo_get",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401],
    },
    SuiteEntry {
        route: "purchases_photo_delete",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401],
    },
    SuiteEntry {
        route: "brews_list",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "brews_create",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "brews_get",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401],
    },
    SuiteEntry {
        route: "brews_update",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "brews_archive",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401],
    },
    SuiteEntry {
        route: "brews_unarchive",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401],
    },
    SuiteEntry {
        route: "suggestions_list",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "stats_brews",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "stats_purchases",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "stats_brew_ratings",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401, KIND_INVALID_INPUT_400],
    },
    SuiteEntry {
        route: "purchases_rating_history",
        kinds: &[KIND_OK, KIND_UNAUTHENTICATED_401],
    },
];

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
        let vars: Vec<(String, String)> = vars
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect();
        Self::start_with(|_| vars.clone(), "")
    }

    /// vars をポートから組み立てて `wrangler dev` を起動し、下ごしらえの SQL を先に実行する。
    ///
    /// WebAuthn の Origin はポートを含むため、vars の組み立てにポートが要る。
    /// 下ごしらえは子プロセスの起動前に実行し、実行中の D1 への同時の書き込みを避ける。
    pub fn start_with(
        vars: impl FnOnce(u16) -> Vec<(String, String)>,
        seed_sql: &str,
    ) -> Result<Self, String> {
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

        apply_migrations(&manifest_dir, &persist_dir)?;
        if !seed_sql.is_empty() {
            execute_sql_at(&manifest_dir, &persist_dir, seed_sql)?;
        }

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
        for (key, value) in vars(port) {
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
        if let Err(error) = server.wait_until_ready() {
            server.stop();
            return Err(error);
        }
        Ok(server)
    }

    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Chrome から開く URL。WebAuthn の RP ID と Origin は、ページのオリジンと一致する必要がある。
    pub fn localhost_url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    /// 使用中のポート。
    pub fn port(&self) -> u16 {
        self.port
    }

    /// この Worker の Origin。vars の `ORIGIN` に渡す値と同じ。
    pub fn origin(&self) -> String {
        self.localhost_url()
    }

    /// `wrangler d1 execute` でローカルの D1 に SQL を実行する。
    /// テストの下ごしらえ (利用者と登録用トークンの投入) と、結果の確認に使う。
    pub fn execute_sql(&self, sql: &str) -> Result<String, String> {
        execute_sql_at(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            &self.persist_dir,
            sql,
        )
    }

    /// 複数の文をまとめて実行する。`wrangler d1 execute --file` に一時ファイルを渡す。
    /// 文ごとに `wrangler` を起動しないため、想定規模のデータの投入に使う。
    pub fn execute_sql_file(&self, statements: &[String]) -> Result<(), String> {
        let path = self.persist_dir.join("bulk.sql");
        std::fs::write(&path, statements.join(";\n"))
            .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
        execute_sql_file_at(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            &self.persist_dir,
            &path,
        )
    }

    /// D1 から 1 つの整数を引く (下ごしらえの結果の確認用)。
    /// 先頭の行の先頭の列を整数として読む。
    pub fn query_int(&self, sql: &str) -> Result<i64, String> {
        let output = self.execute_sql_json(sql)?;
        let parsed: serde_json::Value = serde_json::from_str(&output)
            .map_err(|error| format!("the d1 output must be JSON but was {output}: {error}"))?;
        let value = parsed
            .get(0)
            .and_then(|result| result.get("results"))
            .and_then(|results| results.get(0))
            .and_then(|row| row.as_object())
            .and_then(|row| row.values().next())
            .and_then(|value| value.as_i64());
        value.ok_or_else(|| format!("the d1 output has no integer: {output}"))
    }

    /// `wrangler r2 object put` でローカルの R2 にオブジェクトを置く (0009 の写真の下ごしらえ)。
    /// アップロードの完了通知は、クライアントが置いた `pending/` のオブジェクトを確認するため、
    /// テストは署名付き URL に PUT せず、この操作でその状態を作る。
    pub fn put_r2_object(
        &self,
        bucket: &str,
        key: &str,
        bytes: &[u8],
        content_type: &str,
    ) -> Result<(), String> {
        let path = self.r2_temp_path();
        std::fs::write(&path, bytes)
            .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
        let mut command = r2_object_command(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            &self.persist_dir,
            "put",
            &format!("{bucket}/{key}"),
        );
        command
            .arg("--file")
            .arg(&path)
            .arg("--content-type")
            .arg(content_type)
            // データカタログの確認のプロンプトを出さない。
            .arg("--force");
        let output = command
            .output()
            .map_err(|error| format!("failed to run wrangler r2 object put: {error}"))?;
        let _ = std::fs::remove_file(&path);
        if !output.status.success() {
            return Err(format!(
                "wrangler r2 object put failed with {}\nstdout:\n{}\nstderr:\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    }

    /// `wrangler r2 object get` でローカルの R2 からオブジェクトを読む。無いときは None。
    pub fn get_r2_object(&self, bucket: &str, key: &str) -> Result<Option<Vec<u8>>, String> {
        let path = self.r2_temp_path();
        let mut command = r2_object_command(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            &self.persist_dir,
            "get",
            &format!("{bucket}/{key}"),
        );
        command.arg("--file").arg(&path);
        let output = command
            .output()
            .map_err(|error| format!("failed to run wrangler r2 object get: {error}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let _ = std::fs::remove_file(&path);
            // 無いオブジェクトの取得は失敗の終了状態になる。他の失敗と区別する。
            if stderr.contains("The specified key does not exist.") {
                return Ok(None);
            }
            return Err(format!(
                "wrangler r2 object get failed with {}\nstdout:\n{}\nstderr:\n{stderr}",
                output.status,
                String::from_utf8_lossy(&output.stdout)
            ));
        }
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let _ = std::fs::remove_file(&path);
        Ok(Some(bytes))
    }

    /// R2 の操作に使う一時ファイルのパス。並行に走るテストで衝突しないよう連番を付ける。
    fn r2_temp_path(&self) -> PathBuf {
        let sequence = R2_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        self.persist_dir
            .join(format!("r2-object-{}-{sequence}.bin", std::process::id()))
    }

    /// `--json` を付けて SQL を実行する。
    fn execute_sql_json(&self, sql: &str) -> Result<String, String> {
        execute_sql_json_at(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            &self.persist_dir,
            sql,
        )
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
}

impl Drop for DevServer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// 共有する `wrangler dev` の置き場。借用が残っている間だけ生かすため Weak で持つ。
type SharedSlot = (&'static str, Weak<Mutex<Option<DevServer>>>);

/// テストファイルで共有する `wrangler dev`。`name` ごとに 1 つ持つ。
static SHARED: Mutex<Vec<SharedSlot>> = Mutex::new(Vec::new());

/// テストファイルで共有する `wrangler dev` を借りる。`name` ごとに 1 つ起動する。
/// 最後の借用が落ちると `DevServer` が落ちて子プロセスが停止する。
pub fn shared_server(
    name: &'static str,
    start: impl FnOnce() -> Result<DevServer, String>,
) -> Result<ServerLease, String> {
    // OnceLock はサーバーを永久に保持するため使わない。Weak にしておき、
    // 借用が全て落ちたらサーバーも落ちるようにする。
    let mut slot = SHARED
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(server) = slot
        .iter()
        .find(|(key, _)| *key == name)
        .and_then(|(_, weak)| weak.upgrade())
    {
        return Ok(ServerLease(server));
    }
    let server = start()?;
    let server = Arc::new(Mutex::new(Some(server)));
    slot.retain(|(key, _)| *key != name);
    slot.push((name, Arc::downgrade(&server)));
    Ok(ServerLease(server))
}

/// 共有のサーバーの借用。借用が全て落ちると、共有のサーバーも停止する。
pub struct ServerLease(Arc<Mutex<Option<DevServer>>>);

impl ServerLease {
    /// サーバーを使う。借用中は他のテストを待たせる。
    pub fn use_server<T>(&self, action: impl FnOnce(&DevServer) -> T) -> T {
        let guard = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let server = guard
            .as_ref()
            .expect("the shared server must live while the lease is held");
        action(server)
    }

    /// 借用を直接取る。
    pub fn guard(&self) -> MutexGuard<'_, Option<DevServer>> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
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

/// `wrangler d1 migrations apply` でローカルの D1 にマイグレーションを適用する。
///
/// D1 のバインディングは 0003 が追加する。バインディングが無い間は適用をスキップする。
fn apply_migrations(manifest_dir: &Path, persist_dir: &Path) -> Result<(), String> {
    let config_path = manifest_dir.join("wrangler.toml");
    let config = std::fs::read_to_string(&config_path)
        .map_err(|error| format!("failed to read {}: {error}", config_path.display()))?;
    if !has_d1_binding(&config) {
        return Ok(());
    }
    let status = Command::new("wrangler")
        .args(["d1", "migrations", "apply", "DB", "--local", "--persist-to"])
        .arg(persist_dir)
        .current_dir(manifest_dir)
        .env("WRANGLER_SEND_METRICS", "false")
        .env_remove("OPENCODE")
        .env_remove("AGENT")
        .env_remove("AI_AGENT")
        .env_remove("CLAUDECODE")
        .status()
        .map_err(|error| format!("failed to run wrangler d1 migrations apply: {error}"))?;
    if !status.success() {
        return Err(format!("wrangler d1 migrations apply failed with {status}"));
    }
    Ok(())
}

/// `wrangler d1 execute` で SQL を実行し、標準出力を返す。
fn execute_sql_at(manifest_dir: &Path, persist_dir: &Path, sql: &str) -> Result<String, String> {
    run_d1_execute(manifest_dir, persist_dir, sql, false)
}

/// `wrangler d1 execute --file` でファイルの SQL を実行する。
fn execute_sql_file_at(manifest_dir: &Path, persist_dir: &Path, path: &Path) -> Result<(), String> {
    let mut command = d1_execute_command(manifest_dir, persist_dir);
    command.arg("--file").arg(path);
    let output = command
        .output()
        .map_err(|error| format!("failed to run wrangler d1 execute: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "wrangler d1 execute --file failed with {}\nstdout:\n{}\nstderr:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

/// `wrangler d1 execute --json` で SQL を実行し、標準出力を返す。
fn execute_sql_json_at(
    manifest_dir: &Path,
    persist_dir: &Path,
    sql: &str,
) -> Result<String, String> {
    run_d1_execute(manifest_dir, persist_dir, sql, true)
}

/// `wrangler d1 execute` を実行する。D1 の結果は標準出力に出る。
fn run_d1_execute(
    manifest_dir: &Path,
    persist_dir: &Path,
    sql: &str,
    json: bool,
) -> Result<String, String> {
    let mut command = d1_execute_command(manifest_dir, persist_dir);
    command.arg("--command").arg(sql);
    if json {
        command.arg("--json");
    }
    let output = command
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

/// `wrangler r2 object` の共通の引数を組み立てる。
///
/// `--local` を明示するのは、アカウントのログインの状態に依存させないためである
/// (省くと対話の確認になる)。`--persist-to` は `wrangler dev` と同じ置き場を指す。
fn r2_object_command(
    manifest_dir: &Path,
    persist_dir: &Path,
    subcommand: &str,
    object_path: &str,
) -> Command {
    let mut command = Command::new("wrangler");
    command
        .args(["r2", "object", subcommand, object_path])
        .arg("--local")
        .arg("--persist-to")
        .arg(persist_dir)
        .current_dir(manifest_dir)
        .env("WRANGLER_SEND_METRICS", "false")
        .env_remove("OPENCODE")
        .env_remove("AGENT")
        .env_remove("AI_AGENT")
        .env_remove("CLAUDECODE");
    command
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
