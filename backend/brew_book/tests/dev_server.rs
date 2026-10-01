mod support;

use std::time::{Duration, Instant};

/// 0001 の完了条件: `wrangler dev` の起動とマイグレーションの適用と停止を 1 つのテスト実行で行い、
/// 存在しない経路の 404 と、リクエスト 1 件ごとのログを確認する。
///
/// テスト名は、`wrangler dev` を起動するテストを `backend:test` が名前で除外するための
/// (`--skip wrangler_`)、`wrangler_` プレフィックスを持つ。
#[test]
fn wrangler_dev_server_returns_json_404_and_logs_the_request() {
    let server = support::DevServer::start().expect("wrangler dev must start");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("the HTTP client must build");

    // 起動待ちの GET と区別するため、POST で呼ぶ。
    let response = client
        .post(format!("{}/api/not-implemented", server.base_url()))
        .send()
        .expect("the request must reach the dev server");
    assert_eq!(response.status().as_u16(), 404);

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    assert!(
        content_type.starts_with("application/json"),
        "the error response must be JSON but was {content_type}"
    );

    let body_text = response.text().expect("the 404 body must be readable");
    let body: serde_json::Value =
        serde_json::from_str(&body_text).expect("the 404 body must be JSON");
    assert_eq!(
        body,
        serde_json::json!({"error": {"code": "not_found", "message": "route not found"}})
    );

    let Some(line) = server.wait_for_line("\"method\":\"POST\"", Duration::from_secs(15)) else {
        panic!(
            "a request log line for POST must appear in the wrangler dev output:\n{}",
            server.output().join("\n")
        );
    };
    // リクエスト 1 件につき 1 行だけであること (二重出力の退行を検出する)。
    let post_lines: Vec<String> = server
        .output()
        .into_iter()
        .filter(|line| line.contains("\"method\":\"POST\""))
        .collect();
    assert_eq!(
        post_lines.len(),
        1,
        "exactly one log line must be emitted per request but got:\n{}",
        post_lines.join("\n")
    );

    let log: serde_json::Value =
        serde_json::from_str(line.trim()).expect("the request log line must be JSON");
    let log = log
        .as_object()
        .expect("the request log line must be a JSON object");
    assert_eq!(
        log.get("event").and_then(|value| value.as_str()),
        Some("request")
    );
    assert_eq!(
        log.get("route").and_then(|value| value.as_str()),
        Some("not_found")
    );
    assert_eq!(
        log.get("method").and_then(|value| value.as_str()),
        Some("POST")
    );
    assert_eq!(
        log.get("status").and_then(|value| value.as_u64()),
        Some(404)
    );
    assert!(
        log.get("duration_ms")
            .and_then(|value| value.as_u64())
            .is_some(),
        "the log must carry the duration as a non-negative integer"
    );
    assert_eq!(
        log.len(),
        5,
        "the log must have exactly the five metadata fields"
    );
}

/// 0019 の完了条件: `DevServer` の停止で、`wrangler` の孫 (`workerd`、esbuild) が孤児として残らない。
///
/// `wrangler` は `workerd` と esbuild を子として起動する。停止はプロセスグループごとに行い、
/// グループにプロセスが残っていないことを確認する。
#[test]
fn wrangler_dev_server_stop_leaves_no_process_in_its_process_group() {
    let mut server = support::DevServer::start().expect("wrangler dev must start");
    let process_group_id = server.process_group_id();
    server.stop();

    // SIGKILL と、孤児の回収 (launchd による reap) の反映を待つ。
    let deadline = Instant::now() + Duration::from_secs(10);
    while support::process_group_exists(process_group_id) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        !support::process_group_exists(process_group_id),
        "the process group {process_group_id} must be empty after stop but still has processes"
    );
}
