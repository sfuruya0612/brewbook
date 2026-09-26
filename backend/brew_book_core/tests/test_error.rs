use brew_book_core::error::{body, envelope, ErrorCode};

#[test]
fn codes_and_statuses_follow_the_prd_table() {
    let cases = [
        (ErrorCode::BadRequest, "bad_request", 400),
        (ErrorCode::Unauthorized, "unauthorized", 401),
        (ErrorCode::Forbidden, "forbidden", 403),
        (ErrorCode::NotFound, "not_found", 404),
        (ErrorCode::Conflict, "conflict", 409),
        (ErrorCode::Gone, "gone", 410),
        (ErrorCode::Internal, "internal_error", 500),
    ];
    for (code, expected_code, expected_status) in cases {
        assert_eq!(code.as_str(), expected_code);
        assert_eq!(code.status(), expected_status);
    }
}

#[test]
fn body_has_the_error_envelope_shape() {
    let value: serde_json::Value =
        serde_json::from_str(&body(ErrorCode::NotFound, "route not found")).unwrap();
    assert_eq!(
        value,
        serde_json::json!({"error": {"code": "not_found", "message": "route not found"}})
    );
}

#[test]
fn body_escapes_special_characters_in_the_message() {
    let message = "quote \" backslash \\ newline \n tab \t 日本語 全角";
    let value: serde_json::Value =
        serde_json::from_str(&body(ErrorCode::BadRequest, message)).unwrap();
    assert_eq!(value["error"]["message"], message);
    assert_eq!(value["error"]["code"], "bad_request");
}

#[test]
fn envelope_keeps_the_code_and_message() {
    let envelope = envelope(ErrorCode::Conflict, "already archived");
    assert_eq!(envelope.error.code, "conflict");
    assert_eq!(envelope.error.message, "already archived");
}

#[test]
fn body_is_a_single_line_without_pretty_printing() {
    let body = body(ErrorCode::Unauthorized, "session is missing");
    assert!(!body.contains('\n'));
}
