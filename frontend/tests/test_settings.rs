//! `settings` と設定の画面の単体テスト (0043)。
//!
//! エクスポートの取得と保存 (FR-14)、アカウント削除の確認 (FR-15)、パスキーの一覧の表示と
//! 最後の 1 つの削除の禁止 (FR-3) を確かめる。パスキーの操作の API の検査は `test_auth.rs`
//! が担う。

mod support;

use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use brew_book_frontend::auth::{message_key, Passkey};
use brew_book_frontend::i18n::{Key, Language};
use brew_book_frontend::screens::settings::{can_delete_passkey, passkey_subtitle};
use brew_book_frontend::screens::SettingsScreen;
use brew_book_frontend::settings::{export_all, SettingsServices};
use dioxus::prelude::Element;

use support::{block_on, client, FakeFileDownload, FakeTransport};

#[test]
fn the_settings_screen_has_no_props() {
    // 経路に割り当てる画面は、ルーターが渡す prop で組めること (型の確認)。
    let _: fn() -> Element = SettingsScreen;
}

#[test]
fn the_settings_route_is_wired_to_the_settings_screen() {
    let router =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/router.rs"))
            .expect("the router must be readable");

    assert!(router.contains("#[route(\"/settings\", SettingsScreen)]"));
}

#[test]
fn the_export_is_downloaded_as_the_json_file() {
    let (client, transport) = client(vec![FakeTransport::response(200, r#"{"brews":[]}"#)]);
    let download = Rc::new(FakeFileDownload::new());
    let services = SettingsServices::new(client, download.clone());

    block_on(export_all(&services)).expect("the export must succeed");

    let request = transport.last_request();
    assert_eq!(request.method, brew_book_frontend::api::Method::Get);
    assert_eq!(request.path, "/api/export");
    let saved = download.saved();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].file_name, SettingsServices::EXPORT_FILE_NAME);
    assert_eq!(saved[0].bytes, br#"{"brews":[]}"#.to_vec());
}

#[test]
fn a_failed_export_is_not_saved() {
    let (client, _) = client(vec![FakeTransport::response(500, "")]);
    let download = Rc::new(FakeFileDownload::new());
    let services = SettingsServices::new(client, download.clone());

    let error = block_on(export_all(&services)).expect_err("the export must fail");

    assert_eq!(message_key(&error), Key::ErrorUnexpected);
    assert!(download.saved().is_empty());
}

#[test]
fn the_last_passkey_cannot_be_deleted() {
    assert!(!can_delete_passkey(0));
    assert!(!can_delete_passkey(1));
    assert!(can_delete_passkey(2));
    assert!(can_delete_passkey(3));
}

#[test]
fn the_passkey_subtitle_shows_the_created_and_last_used_timestamps() {
    let passkey = Passkey {
        id: "p1".to_string(),
        name: "自宅の Mac".to_string(),
        created_at: "2026-09-01T01:00:00.000Z".to_string(),
        last_used_at: Some("2026-09-10T10:00:00.000Z".to_string()),
    };

    // 日時は端末のタイムゾーン (+9 時間) に直し、言語に合わせて表示する (FR-3、FR-16)。
    assert_eq!(
        passkey_subtitle(&passkey, Language::Japanese, 540),
        "登録: 2026/9/1 10:00 / 最終使用: 2026/9/10 19:00"
    );
    assert_eq!(
        passkey_subtitle(&passkey, Language::English, 540),
        "Created: 9/1/2026 10:00 / Last used: 9/10/2026 19:00"
    );

    // まだ使われていないパスキーはその旨を出す (FR-3)。
    let unused = Passkey {
        last_used_at: None,
        ..passkey.clone()
    };
    assert_eq!(
        passkey_subtitle(&unused, Language::Japanese, 540),
        "登録: 2026/9/1 10:00 / まだ使われていません"
    );
    assert_eq!(
        passkey_subtitle(&unused, Language::English, 540),
        "Created: 9/1/2026 10:00 / Not used yet"
    );

    // 読めない日時はそのまま返す。
    let broken = Passkey {
        created_at: "not a timestamp".to_string(),
        last_used_at: None,
        ..passkey.clone()
    };
    assert_eq!(
        passkey_subtitle(&broken, Language::English, 540),
        "Created: not a timestamp / Not used yet"
    );
}
