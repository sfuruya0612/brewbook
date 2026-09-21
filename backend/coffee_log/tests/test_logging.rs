use coffee_log::logging::{line, RequestLog};

#[test]
fn log_line_contains_only_the_route_metadata() {
    let log = RequestLog::new("shops_list", "GET", 200, 42);
    let value: serde_json::Value = serde_json::from_str(&line(&log)).unwrap();
    assert_eq!(
        value,
        serde_json::json!({
            "event": "request",
            "route": "shops_list",
            "method": "GET",
            "status": 200,
            "duration_ms": 42
        })
    );
}

#[test]
fn log_line_has_exactly_the_five_metadata_fields() {
    let log = RequestLog::new("not_found", "POST", 404, 7);
    let value: serde_json::Value = serde_json::from_str(&line(&log)).unwrap();
    let mut keys: Vec<&str> = value
        .as_object()
        .expect("a log line is a JSON object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec!["duration_ms", "event", "method", "route", "status"]
    );
}
